use std::{
    collections::HashMap,
    sync::Arc,
    time::Instant,
};

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;

use crate::{
    claude::{
        client::ClaudeClient,
        message::Message,
    },
    error::AgentFlowError,
    observability::{
        event::{EventPayload, PipelineEvent},
        hub::EventHub,
    },
    tools::traits::Tool,
};

use super::traits::{Agent, AgentContext, AgentOutput, QAIssue};

/// Structured JSON format expected from QA / reviewer agents.
#[derive(Debug, serde::Deserialize)]
struct QAResponse {
    passed: bool,
    #[serde(default)]
    issues: Vec<QAIssue>,
}

/// Default agent runner — uses the Claude Messages API, handles tool calls,
/// and parses structured QA output from reviewer agents.
pub struct ClaudeAgentRunner {
    pub(crate) claude: Arc<ClaudeClient>,
    pub(crate) tools: Arc<HashMap<String, Arc<dyn Tool>>>,
    pub(crate) hub: Arc<EventHub>,
    pub(crate) run_id: String,
}

impl ClaudeAgentRunner {
    pub fn new(
        claude: Arc<ClaudeClient>,
        tools: Arc<HashMap<String, Arc<dyn Tool>>>,
        hub: Arc<EventHub>,
        run_id: String,
    ) -> Self {
        Self {
            claude,
            tools,
            hub,
            run_id,
        }
    }

    /// Run a single agent invocation (including the tool-calling loop if needed).
    pub async fn run_with_id(
        &self,
        ctx: &AgentContext,
        agent_id: &str,
    ) -> Result<AgentOutput, AgentFlowError> {
        let start = Instant::now();

        // Emit agent_started
        self.hub
            .emit(PipelineEvent {
                run_id: self.run_id.clone(),
                timestamp: Utc::now(),
                payload: EventPayload::AgentStarted {
                    agent_id: agent_id.to_string(),
                    attempt: ctx.attempt,
                    module_id: None,
                },
            })
            .await;

        let user_msg = ctx.build_user_message();
        let initial_messages = vec![Message::user(user_msg)];

        // Build tool definitions for tools declared on this agent
        let available_tools: Vec<(String, String, Value)> = ctx
            .tool_ids
            .iter()
            .filter_map(|id| self.tools.get(id.as_str()))
            .map(|t| (t.id().to_string(), t.description().to_string(), t.input_schema()))
            .collect();

        let tool_defs = if available_tools.is_empty() {
            None
        } else {
            Some(available_tools.clone())
        };

        // Agentic loop — resolves tool calls before returning final text
        let hub = self.hub.clone();
        let tools_map = self.tools.clone();
        let run_id = self.run_id.clone();
        let agent_id_owned = agent_id.to_string();

        let text = self
            .claude
            .agentic_loop(
                &ctx.model,
                ctx.max_tokens,
                Some(&ctx.system_prompt),
                initial_messages,
                tool_defs,
                |call| {
                    let hub = hub.clone();
                    let tools_map = tools_map.clone();
                    let run_id = run_id.clone();
                    let agent_id_c = agent_id_owned.clone();
                    async move {
                        // Emit tool_called
                        hub.emit(PipelineEvent {
                            run_id: run_id.clone(),
                            timestamp: Utc::now(),
                            payload: EventPayload::ToolCalled {
                                agent_id: agent_id_c.clone(),
                                tool_id: call.name.clone(),
                            },
                        })
                        .await;

                        let result = if let Some(tool) = tools_map.get(&call.name) {
                            tool.execute(call.input).await
                        } else {
                            Err(AgentFlowError::Tool(format!(
                                "tool '{}' not found",
                                call.name
                            )))
                        };

                        let (success, result_str) = match &result {
                            Ok(v) => (true, v.to_string()),
                            Err(e) => (false, format!("{{\"error\":\"{}\"}}", e)),
                        };

                        // Emit tool_result
                        hub.emit(PipelineEvent {
                            run_id: run_id.clone(),
                            timestamp: Utc::now(),
                            payload: EventPayload::ToolResult {
                                agent_id: agent_id_c.clone(),
                                tool_id: call.name.clone(),
                                success,
                            },
                        })
                        .await;

                        result.map(|_| result_str)
                    }
                },
            )
            .await?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Try to parse as QA JSON response; fall back to plain text.
        let output = Self::parse_output(&text);

        // Emit agent_completed
        self.hub
            .emit(PipelineEvent {
                run_id: self.run_id.clone(),
                timestamp: Utc::now(),
                payload: EventPayload::AgentCompleted {
                    agent_id: agent_id.to_string(),
                    passed: output.passed,
                    duration_ms,
                    module_id: None,
                },
            })
            .await;

        Ok(output)
    }

    fn parse_output(text: &str) -> AgentOutput {
        // Extract JSON from the response if wrapped in a code block
        let json_candidate = extract_json(text).unwrap_or(text);

        if let Ok(qa) = serde_json::from_str::<QAResponse>(json_candidate) {
            AgentOutput {
                passed: qa.passed,
                content: text.to_string(),
                issues: qa.issues,
            }
        } else {
            AgentOutput {
                passed: true,
                content: text.to_string(),
                issues: vec![],
            }
        }
    }
}

/// Extract JSON object/array from text that may contain markdown code fences.
fn extract_json(text: &str) -> Option<&str> {
    // Try ```json ... ``` fence
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim());
        }
    }
    // Try ``` ... ``` fence
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        if let Some(end) = after.find("```") {
            let candidate = after[..end].trim();
            if candidate.starts_with('{') || candidate.starts_with('[') {
                return Some(candidate);
            }
        }
    }
    // Try raw JSON in the text
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                return Some(&text[start..=end]);
            }
        }
    }
    None
}

#[async_trait]
impl Agent for ClaudeAgentRunner {
    async fn run(&self, ctx: &AgentContext) -> Result<AgentOutput, AgentFlowError> {
        self.run_with_id(ctx, &ctx.agent_id.clone()).await
    }
}
