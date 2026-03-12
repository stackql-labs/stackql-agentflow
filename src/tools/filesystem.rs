use async_trait::async_trait;
use serde_json::{json, Value};

use crate::error::AgentFlowError;

use super::traits::Tool;

/// Built-in tool that gives agents read access to local files.
pub struct FilesystemTool;

#[async_trait]
impl Tool for FilesystemTool {
    fn id(&self) -> &str {
        "filesystem"
    }

    fn description(&self) -> &str {
        "Read a file from the local filesystem and return its contents as a string."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or relative path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value, AgentFlowError> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentFlowError::Tool("missing 'path' field".to_string()))?;

        let contents = tokio::fs::read_to_string(path).await.map_err(|e| {
            AgentFlowError::Tool(format!("failed to read '{}': {}", path, e))
        })?;

        Ok(json!({ "contents": contents, "path": path }))
    }
}
