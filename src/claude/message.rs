use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// A single content block inside a message (text, tool_use, or tool_result).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,

    // text block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    // tool_use block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,

    // tool_result block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

impl ContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: Some(text.into()),
            id: None,
            name: None,
            input: None,
            tool_use_id: None,
            content: None,
        }
    }

    pub fn tool_use(
        id: impl Into<String>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self {
            block_type: "tool_use".to_string(),
            text: None,
            id: Some(id.into()),
            name: Some(name.into()),
            input: Some(input),
            tool_use_id: None,
            content: None,
        }
    }

    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            block_type: "tool_result".to_string(),
            text: None,
            id: None,
            name: None,
            input: None,
            tool_use_id: Some(tool_use_id.into()),
            content: Some(content.into()),
        }
    }
}

/// A message in the conversation, either simple text or a list of content blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(text.into()),
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(text.into()),
        }
    }

    pub fn user_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Blocks(blocks),
        }
    }

    pub fn assistant_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Blocks(blocks),
        }
    }
}
