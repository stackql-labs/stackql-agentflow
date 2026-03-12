use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AgentFlowError;

use super::message::{ContentBlock, Message, MessageContent, Role};

/// A pending tool invocation returned by Claude.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// Either a final text response or a list of tool calls that must be resolved.
#[derive(Debug)]
pub enum ClaudeResponse {
    Text(String),
    ToolUse(Vec<ToolCall>),
}

#[derive(Debug, Serialize)]
struct ToolDefinition {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClaudeClient {
    client: Client,
    api_key: String,
}

impl ClaudeClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
        }
    }

    /// Simple text completion with no tools.
    pub async fn complete(
        &self,
        model: &str,
        max_tokens: u32,
        system: Option<&str>,
        messages: Vec<Message>,
    ) -> Result<ClaudeResponse, AgentFlowError> {
        self.send(model, max_tokens, system, messages, None).await
    }

    /// Completion with optional tool definitions.
    pub async fn send(
        &self,
        model: &str,
        max_tokens: u32,
        system: Option<&str>,
        messages: Vec<Message>,
        tools: Option<Vec<(String, String, Value)>>, // (id, description, schema)
    ) -> Result<ClaudeResponse, AgentFlowError> {
        let tool_defs = tools.map(|ts| {
            ts.into_iter()
                .map(|(name, description, input_schema)| ToolDefinition {
                    name,
                    description,
                    input_schema,
                })
                .collect::<Vec<_>>()
        });

        let request = MessagesRequest {
            model: model.to_string(),
            max_tokens,
            system: system.map(|s| s.to_string()),
            messages,
            tools: tool_defs,
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AgentFlowError::AnthropicApi { status, body });
        }

        let resp: MessagesResponse = response.json().await?;

        // Detect tool_use stop reason
        let has_tool_use = resp
            .stop_reason
            .as_deref()
            .map(|r| r == "tool_use")
            .unwrap_or(false);

        if has_tool_use {
            let calls = resp
                .content
                .iter()
                .filter(|b| b.block_type == "tool_use")
                .filter_map(|b| {
                    Some(ToolCall {
                        id: b.id.clone()?,
                        name: b.name.clone()?,
                        input: b.input.clone().unwrap_or(Value::Null),
                    })
                })
                .collect::<Vec<_>>();

            if !calls.is_empty() {
                return Ok(ClaudeResponse::ToolUse(calls));
            }
        }

        let text = resp
            .content
            .iter()
            .filter(|b| b.block_type == "text")
            .filter_map(|b| b.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("");

        Ok(ClaudeResponse::Text(text))
    }

    /// Run a full agentic loop: send messages, handle tool calls, and return the final text.
    ///
    /// `tool_resolver` is called with each ToolCall and should return the result string.
    pub async fn agentic_loop<F, Fut>(
        &self,
        model: &str,
        max_tokens: u32,
        system: Option<&str>,
        mut messages: Vec<Message>,
        tools: Option<Vec<(String, String, Value)>>,
        mut tool_resolver: F,
    ) -> Result<String, AgentFlowError>
    where
        F: FnMut(ToolCall) -> Fut,
        Fut: std::future::Future<Output = Result<String, AgentFlowError>>,
    {
        loop {
            match self
                .send(model, max_tokens, system, messages.clone(), tools.clone())
                .await?
            {
                ClaudeResponse::Text(text) => return Ok(text),
                ClaudeResponse::ToolUse(calls) => {
                    // Add the assistant tool_use blocks to the conversation
                    let assistant_blocks: Vec<ContentBlock> = calls
                        .iter()
                        .map(|c| {
                            ContentBlock::tool_use(c.id.clone(), c.name.clone(), c.input.clone())
                        })
                        .collect();
                    messages.push(Message {
                        role: Role::Assistant,
                        content: MessageContent::Blocks(assistant_blocks),
                    });

                    // Resolve each tool call and collect results
                    let mut result_blocks: Vec<ContentBlock> = Vec::new();
                    for call in calls {
                        let tool_id = call.id.clone();
                        let result = tool_resolver(call).await?;
                        result_blocks.push(ContentBlock::tool_result(tool_id, result));
                    }

                    // Add the tool results as a user message
                    messages.push(Message {
                        role: Role::User,
                        content: MessageContent::Blocks(result_blocks),
                    });
                }
            }
        }
    }
}
