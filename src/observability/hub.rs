use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::error;

use super::{event::PipelineEvent, sink::LogSink};

const DEFAULT_CAPACITY: usize = 1024;

/// Central broadcast channel for pipeline events.
///
/// All runtime components emit events here; the hub fans them out to every
/// registered `LogSink` and to any WebSocket subscribers.
#[derive(Clone)]
pub struct EventHub {
    sender: broadcast::Sender<PipelineEvent>,
}

impl EventHub {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(DEFAULT_CAPACITY);
        Self { sender }
    }

    /// Subscribe to the broadcast stream. Each subscriber gets every event
    /// published after the subscription was created.
    pub fn subscribe(&self) -> broadcast::Receiver<PipelineEvent> {
        self.sender.subscribe()
    }

    /// Publish an event to all subscribers.  Silently drops if no receivers.
    pub async fn emit(&self, event: PipelineEvent) {
        let _ = self.sender.send(event);
    }

    /// Spawn a background task that forwards every event to all `sinks`.
    pub fn start_sinks(&self, sinks: Vec<Box<dyn LogSink + Send + Sync>>) {
        if sinks.is_empty() {
            return;
        }
        let mut rx = self.subscribe();
        let sinks: Arc<Vec<Box<dyn LogSink + Send + Sync>>> = Arc::new(sinks);

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        for sink in sinks.as_ref() {
                            if let Err(e) = sink.emit(&event).await {
                                error!("log sink error: {}", e);
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        error!("event hub: lagged {} events", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }
}

impl Default for EventHub {
    fn default() -> Self {
        Self::new()
    }
}
