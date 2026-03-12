use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Every structured event emitted during a pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEvent {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub payload: EventPayload,
}

/// All possible event types, each carrying its own fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayload {
    PipelineStarted {
        pipeline_name: String,
    },
    PipelineCompleted {
        total_modules: usize,
        duration_ms: u64,
    },
    PipelineFailed {
        reason: String,
    },
    StateTransition {
        from: String,
        to: String,
    },
    AgentStarted {
        agent_id: String,
        attempt: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        module_id: Option<String>,
    },
    AgentCompleted {
        agent_id: String,
        passed: bool,
        duration_ms: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        module_id: Option<String>,
    },
    FeedbackIssued {
        qa_agent_id: String,
        target_agent_id: String,
        issue_count: usize,
    },
    ToolCalled {
        agent_id: String,
        tool_id: String,
    },
    ToolResult {
        agent_id: String,
        tool_id: String,
        success: bool,
    },
    Log {
        level: String,
        message: String,
    },
}
