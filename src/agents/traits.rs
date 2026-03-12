use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::AgentFlowError;

/// A structured quality issue returned by a QA agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAIssue {
    /// `blocking` (must be fixed) or `warning` (should be addressed)
    pub severity: String,
    pub description: String,
    pub suggestion: String,
}

/// The result of a single agent invocation.
#[derive(Debug, Clone)]
pub struct AgentOutput {
    /// Whether the agent considers its work complete / passing.
    pub passed: bool,
    /// The agent's primary text output (written content, analysis, etc.)
    pub content: String,
    /// Structured issues from a QA agent. Empty for non-QA agents.
    pub issues: Vec<QAIssue>,
}

/// Execution context passed into every agent invocation.
#[derive(Debug, Clone)]
pub struct AgentContext {
    pub agent_id: String,
    /// Full system prompt read from the prompt file.
    pub system_prompt: String,
    /// Primary user input for this invocation.
    pub user_input: String,
    /// Feedback issues injected from a QA agent on previous attempt.
    pub feedback: Vec<QAIssue>,
    /// Current attempt number (1-based).
    pub attempt: u32,
    /// Tool IDs available for this invocation.
    pub tool_ids: Vec<String>,
    pub model: String,
    pub max_tokens: u32,
}

/// Core agent interface. Implement this to create custom agent runners.
#[async_trait]
pub trait Agent: Send + Sync {
    async fn run(&self, ctx: &AgentContext) -> Result<AgentOutput, AgentFlowError>;
}
