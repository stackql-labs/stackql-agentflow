use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentFlowError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("State machine error: {0}")]
    State(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("Max retries exceeded for agent '{agent_id}'")]
    MaxRetriesExceeded { agent_id: String },

    #[error("Anthropic API error (HTTP {status}): {body}")]
    AnthropicApi { status: u16, body: String },

    #[error("Pipeline failed: {0}")]
    PipelineFailed(String),
}
