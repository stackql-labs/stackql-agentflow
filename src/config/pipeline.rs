use serde::{Deserialize, Serialize};

use super::{agent::AgentConfig, state::StateMachineConfig, tool::ToolConfig};

/// Default values applied to all agents that do not specify their own.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// Aggregation gate: how to decide when the pipeline is complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationAction {
    pub transition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    /// `all_pass`, `any_pass`, or `threshold`
    pub strategy: String,
    /// Agent IDs that act as gates
    pub gates: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_pass: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_complete: Option<AggregationAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_any_abort: Option<AggregationAction>,
}

/// The top-level pipeline definition, deserialized from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub tools: Vec<ToolConfig>,
    pub state_machine: StateMachineConfig,
    pub agents: Vec<AgentConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregation: Option<AggregationConfig>,
}

impl PipelineConfig {
    /// Parse a `PipelineConfig` from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Parse a `PipelineConfig` from a YAML file on disk.
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let yaml = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&yaml)?)
    }
}
