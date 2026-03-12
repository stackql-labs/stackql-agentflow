use async_trait::async_trait;
use tokio::{fs::OpenOptions, io::AsyncWriteExt, sync::Mutex};

use crate::error::AgentFlowError;

use super::event::PipelineEvent;

/// Any component that wants to consume pipeline events implements this trait.
///
/// Sinks are registered on the pipeline before `run()` and receive every event
/// through the `EventHub` broadcast channel.
#[async_trait]
pub trait LogSink: Send + Sync {
    async fn emit(&self, event: &PipelineEvent) -> Result<(), AgentFlowError>;
}

// ---------------------------------------------------------------------------
// ConsoleSink
// ---------------------------------------------------------------------------

/// Prints every event as a single JSON line to stdout.
pub struct ConsoleSink;

#[async_trait]
impl LogSink for ConsoleSink {
    async fn emit(&self, event: &PipelineEvent) -> Result<(), AgentFlowError> {
        let json = serde_json::to_string(event)?;
        println!("{}", json);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// FileSink
// ---------------------------------------------------------------------------

/// Appends every event as JSONL to a file for audit retention.
pub struct FileSink {
    #[allow(dead_code)]
    path: String,
    file: Mutex<tokio::fs::File>,
}

impl FileSink {
    pub async fn new(path: impl Into<String>) -> Result<Self, AgentFlowError> {
        let path = path.into();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;
        Ok(Self {
            path,
            file: Mutex::new(file),
        })
    }
}

#[async_trait]
impl LogSink for FileSink {
    async fn emit(&self, event: &PipelineEvent) -> Result<(), AgentFlowError> {
        let mut line = serde_json::to_string(event)?;
        line.push('\n');
        let mut f = self.file.lock().await;
        f.write_all(line.as_bytes()).await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// WebhookSink
// ---------------------------------------------------------------------------

/// POSTs every event as JSON to an HTTP endpoint.
pub struct WebhookSink {
    endpoint: String,
    client: reqwest::Client,
}

impl WebhookSink {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LogSink for WebhookSink {
    async fn emit(&self, event: &PipelineEvent) -> Result<(), AgentFlowError> {
        self.client
            .post(&self.endpoint)
            .json(event)
            .send()
            .await
            .map_err(AgentFlowError::Http)?;
        Ok(())
    }
}
