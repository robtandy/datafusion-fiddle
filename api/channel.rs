use std::sync::Arc;

// For now, return None since distributed mode requires more complex setup
// You can implement this later based on the datafusion-distributed requirements
pub fn create_channel_resolver() -> Option<Arc<dyn Send + Sync>> {
    None
}