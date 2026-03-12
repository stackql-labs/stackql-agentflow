use async_trait::async_trait;
use serde_json::Value;

use crate::error::AgentFlowError;

/// Every tool — built-in or plugin — implements this trait.
///
/// Tools are registered on a `Pipeline` before `run()` is called and can be
/// listed in the agent's YAML `tools:` array. When an agent triggers a tool
/// call Claude returns, the runtime resolves the matching tool by `id()`,
/// calls `execute()`, and injects the result back into the conversation.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique identifier used in YAML and by Claude as the function name.
    fn id(&self) -> &str;

    /// Human-readable description sent to Claude in the tool definition.
    fn description(&self) -> &str;

    /// JSON Schema describing the `input` object Claude must supply.
    fn input_schema(&self) -> Value;

    /// Execute the tool and return a JSON result or an error.
    async fn execute(&self, input: Value) -> Result<Value, AgentFlowError>;
}
