use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How a tool is provided: by the framework (`builtin`) or by user code (`plugin`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    Builtin,
    Plugin,
}

/// Tool declaration in the pipeline YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: ToolType,
    /// Fully-qualified Rust type path for plugin tools (informational / future use)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    /// Arbitrary key-value config passed to the tool
    #[serde(default)]
    pub config: HashMap<String, String>,
}
