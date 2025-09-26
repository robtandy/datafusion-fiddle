use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde_json::{json, Value};

mod core;
use core::{SqlRequest, execute_statements};


#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    run(service_fn(handler)).await
}

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    // Extract the HTTP request from the API Gateway event
    let (payload, _context) = event.into_parts();

    tracing::info!("Incoming Lambda event: {}", serde_json::to_string(&payload)?);

    // Parse the path
    let path = payload["path"].as_str().unwrap_or("");

    if path != "/api/sql" {
        let response = json!({
            "statusCode": 404,
            "headers": {
                "Content-Type": "application/json"
            },
            "body": json!({"message": "Not Found"}).to_string()
        });
        tracing::info!("404 response: {}", serde_json::to_string(&response)?);
        return Ok(response);
    }

    // Parse the body
    let body_str = payload["body"].as_str().unwrap_or("{}");
    tracing::info!("Request body: {}", body_str);
    let sql_request: SqlRequest = match serde_json::from_str(body_str) {
        Ok(req) => req,
        Err(e) => {
            let response = json!({
                "statusCode": 400,
                "headers": {
                    "Content-Type": "application/json",
                    "Access-Control-Allow-Origin": "*",
                    "Access-Control-Allow-Methods": "POST, OPTIONS",
                    "Access-Control-Allow-Headers": "Content-Type"
                },
                "body": json!({ "message": format!("Invalid JSON: {}", e) }).to_string()
            });
            tracing::error!("JSON parse error response: {}", serde_json::to_string(&response)?);
            return Ok(response);
        }
    };

    // Execute SQL
    let parquet_path = std::env::var("PARQUET_PATH").unwrap_or_else(|_| "/tmp/parquet".to_string());
    tracing::info!("Using PARQUET_PATH environment variable: {}", parquet_path);

    let result = match execute_statements(
        sql_request.stmts,
        &parquet_path,
        sql_request.distributed,
        None,  // Simplified: no distributed mode for now
    ).await {
        Ok(res) => res,
        Err(err) => {
            let response = json!({
                "statusCode": 400,
                "headers": {
                    "Content-Type": "application/json",
                    "Access-Control-Allow-Origin": "*",
                    "Access-Control-Allow-Methods": "POST, OPTIONS",
                    "Access-Control-Allow-Headers": "Content-Type"
                },
                "body": json!({ "message": err.to_string() }).to_string()
            });
            tracing::error!("SQL execution error response: {}", serde_json::to_string(&response)?);
            return Ok(response);
        }
    };

    // Return successful response
    let response = json!({
        "statusCode": 200,
        "headers": {
            "Content-Type": "application/json",
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": "POST, OPTIONS",
            "Access-Control-Allow-Headers": "Content-Type"
        },
        "body": serde_json::to_string(&result)?
    });
    tracing::info!("Success response: {}", serde_json::to_string(&response)?);
    Ok(response)
}