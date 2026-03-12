use serde::{Deserialize, Serialize};

/// Dispatch strategy for sending output to downstream agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchConfig {
    /// `sequential`, `parallel_fan_out`, or `parallel_broadcast`
    pub strategy: String,
    /// Single target agent ID (sequential / broadcast to one)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Multiple target agent IDs (broadcast / fan-out)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
}

/// What to do when a QA agent fails and has retries remaining.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryOnFail {
    /// `feedback_and_retry` or `abort`
    pub action: String,
    /// Agent to re-run with injected feedback
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// JSONPath into issues array (unused in runtime, kept for documentation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback_path: Option<String>,
}

/// Retry policy for a QA agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_fail: Option<RetryOnFail>,
}

/// State-machine transitions triggered by agent lifecycle events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransitionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_complete: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_pass: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_fail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_abort: Option<String>,
}

/// Configuration for a single agent in the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    /// Path to the markdown prompt file
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispatch: Option<DispatchConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub transitions: TransitionConfig,
}
