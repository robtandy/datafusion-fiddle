use datafusion::arrow::error::ArrowError;
use datafusion::arrow::util::display::{ArrayFormatter, FormatOptions};
use datafusion::error::DataFusionError;
use datafusion::execution::SessionStateBuilder;
use datafusion::physical_plan::{displayable, execute_stream};
use datafusion::prelude::{ParquetReadOptions, SessionConfig, SessionContext};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs;
use std::sync::Arc;

const MAX_RESULTS: usize = 500;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct SqlRequest {
    pub distributed: bool,
    pub stmts: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SqlResult {
    pub columns: Vec<(String, String)>,
    pub rows: Vec<Vec<String>>,
    pub logical_plan: String,
    pub physical_plan: String,
}

pub async fn execute_statements(
    stmts: Vec<String>,
    path: impl Display,
    _distributed: bool,  // Ignore distributed mode for now
    _channel_resolver: Option<Arc<dyn Send + Sync>>,
) -> datafusion::error::Result<SqlResult> {
    tracing::info!("execute_statements called with {} statements, path: {}, distributed: {}",
                   stmts.len(), path, _distributed);
    tracing::debug!("Statements: {:?}", stmts);
    let options = FormatOptions::default().with_display_error(true);
    let cfg = SessionConfig::new().with_information_schema(true);

    let builder = SessionStateBuilder::new()
        .with_default_features()
        .with_config(cfg);

    // Note: Distributed mode would require additional setup here
    // For now, we'll run in single-node mode

    let ctx = Arc::new(SessionContext::new_with_state(builder.build()));
    let parquet_path = path.to_string();
    tracing::info!("Using parquet path from parameter: {}", parquet_path);
    load_parquet_files(parquet_path, &ctx).await?;

    if stmts.is_empty() {
        return Ok(SqlResult::default());
    }

    for i in 0..stmts.len() - 1 {
        ctx.sql(stmts.get(i).unwrap()).await?.collect().await?;
    }
    let df = ctx.sql(stmts.last().unwrap()).await?;
    let logical_plan_str = df.logical_plan().display_indent().to_string();

    let physical_plan = df.create_physical_plan().await?;

    let record_batches = execute_stream(physical_plan.clone(), ctx.task_ctx())?
        .try_collect::<Vec<_>>()
        .await?;

    let mut columns: Vec<(String, String)> = vec![];
    let mut rows: Vec<Vec<String>> = vec![];
    for record_batch in record_batches {
        if columns.is_empty() {
            columns = record_batch
                .schema()
                .fields()
                .iter()
                .map(|f| (f.name().clone(), f.data_type().to_string()))
                .collect();
        }
        let formatters = record_batch
            .columns()
            .iter()
            .map(|c| ArrayFormatter::try_new(c.as_ref(), &options))
            .collect::<Result<Vec<_>, ArrowError>>()?;

        for row in 0..record_batch.num_rows() {
            let mut cols = vec![];
            for formatter in &formatters {
                cols.push(formatter.value(row).to_string());
            }
            rows.push(cols);
        }
    }

    let physical_plan_str = displayable(physical_plan.as_ref()).indent(false).to_string();

    Ok(SqlResult {
        columns,
        rows: rows.into_iter().take(MAX_RESULTS).collect(),
        logical_plan: logical_plan_str,
        physical_plan: physical_plan_str,
    })
}

async fn load_parquet_files(path: String, ctx: &SessionContext) -> datafusion::error::Result<()> {
    tracing::info!("Loading parquet files from path: {}", path);

    // Check if path exists, if not, skip loading
    if !std::path::Path::new(&path).exists() {
        tracing::warn!("Parquet path does not exist: {}", path);
        return Ok(());
    }

    tracing::info!("Path exists, reading directory: {}", path);
    let dir = match fs::read_dir(&path) {
        Ok(dir) => dir,
        Err(e) => {
            tracing::error!("Failed to read directory {}: {}", path, e);
            return Err(DataFusionError::External(format!("Failed to read directory {}: {}", path, e).into()));
        }
    };

    let mut table_count = 0;
    for entry in dir {
        let entry = entry?;
        let file_path = entry.path();
        tracing::debug!("Found entry: {:?}", file_path);

        if file_path.is_dir() {
            tracing::info!("Found subdirectory: {:?}, scanning for parquet files...", file_path);
            // Recursively scan subdirectories for parquet files
            let subdir = match fs::read_dir(&file_path) {
                Ok(subdir) => subdir,
                Err(e) => {
                    tracing::warn!("Failed to read subdirectory {:?}: {}", file_path, e);
                    continue;
                }
            };

            for sub_entry in subdir {
                let sub_entry = sub_entry?;
                let sub_path = sub_entry.path();
                tracing::debug!("Found file in subdirectory: {:?}", sub_path);

                if sub_path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                    let table_name = file_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .ok_or_else(|| DataFusionError::External("Invalid directory name".into()))?;

                    tracing::info!("Registering parquet table '{}' from directory: {:?}", table_name, file_path);
                    ctx.register_parquet(table_name, file_path.to_str().unwrap(), ParquetReadOptions::default())
                        .await?;
                    table_count += 1;
                    break; // Only register the directory once, not each file
                }
            }
        } else if file_path.extension().and_then(|s| s.to_str()) == Some("parquet") {
            let table_name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| DataFusionError::External("Invalid file name".into()))?;

            tracing::info!("Registering parquet table '{}' from file: {:?}", table_name, file_path);
            ctx.register_parquet(table_name, file_path.to_str().unwrap(), ParquetReadOptions::default())
                .await?;
            table_count += 1;
        }
    }

    tracing::info!("Successfully loaded {} parquet tables", table_count);
    Ok(())
}