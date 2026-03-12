use std::{collections::HashMap, sync::Arc, time::Instant};

use chrono::Utc;
use tokio::sync::Mutex;

use crate::{
    claude::ClaudeClient,
    config::PipelineConfig,
    error::AgentFlowError,
    observability::{
        event::{EventPayload, PipelineEvent},
        hub::EventHub,
        sink::LogSink,
    },
    orchestrator::Dispatcher,
    retry::RetryGovernor,
    state::StateMachine,
    tools::traits::Tool,
};

/// The main entry point for running a pipeline.
///
/// Build with [`Pipeline::from_yaml`] or [`Pipeline::from_file`], register
/// tools and sinks, then call [`Pipeline::run`].
pub struct Pipeline {
    config: PipelineConfig,
    claude: Arc<ClaudeClient>,
    tools: HashMap<String, Arc<dyn Tool>>,
    sinks: Vec<Box<dyn LogSink + Send + Sync>>,
    hub: Arc<EventHub>,
    obs_port: u16,
}

impl Pipeline {
    /// Parse a pipeline from a YAML string and an Anthropic API key.
    pub fn from_yaml(yaml: &str, api_key: &str) -> Result<Self, AgentFlowError> {
        let config = PipelineConfig::from_yaml(yaml)?;
        Ok(Self::with_config(config, api_key))
    }

    /// Parse a pipeline from a YAML file on disk and an Anthropic API key.
    pub fn from_file(path: &str, api_key: &str) -> Result<Self, AgentFlowError> {
        let yaml = std::fs::read_to_string(path)?;
        Self::from_yaml(&yaml, api_key)
    }

    fn with_config(config: PipelineConfig, api_key: &str) -> Self {
        Self {
            config,
            claude: Arc::new(ClaudeClient::new(api_key)),
            tools: HashMap::new(),
            sinks: Vec::new(),
            hub: Arc::new(EventHub::new()),
            obs_port: 4000,
        }
    }

    /// Override the observability server port (default: 4000).
    pub fn with_obs_port(mut self, port: u16) -> Self {
        self.obs_port = port;
        self
    }

    /// Register a built-in or plugin tool by value.
    pub fn register_tool<T: Tool + 'static>(&mut self, tool: T) {
        self.tools
            .insert(tool.id().to_string(), Arc::new(tool));
    }

    /// Register a log sink.
    pub fn register_sink(&mut self, sink: Box<dyn LogSink + Send + Sync>) {
        self.sinks.push(sink);
    }

    /// Get a clone of the `EventHub` (e.g. to subscribe before calling `run`).
    pub fn hub(&self) -> Arc<EventHub> {
        self.hub.clone()
    }

    // -----------------------------------------------------------------------
    // Run
    // -----------------------------------------------------------------------

    /// Execute the pipeline with the given `initial_input` and block until
    /// the pipeline reaches a terminal state.
    ///
    /// - Starts the observability server on [`Self::obs_port`]
    /// - Emits `pipeline_started` / `pipeline_completed` / `pipeline_failed`
    /// - Returns `Ok(())` on success, `Err(AgentFlowError)` on failure
    pub async fn run(&mut self, initial_input: &str) -> Result<(), AgentFlowError> {
        let run_id = uuid::Uuid::new_v4().to_string();
        let start = Instant::now();

        // Hand off sinks to the hub (drains self.sinks)
        let sinks = std::mem::take(&mut self.sinks);
        self.hub.start_sinks(sinks);

        // Start the observability server in the background
        {
            let hub = self.hub.clone();
            let config = self.config.clone();
            let port = self.obs_port;
            tokio::spawn(async move {
                if let Err(e) = crate::observability::server::start(hub, config, port).await {
                    tracing::error!("observability server: {}", e);
                }
            });
        }

        // Give the server a moment to bind
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        eprintln!(
            "\nobservability UI  ->  http://localhost:{}\n",
            self.obs_port
        );

        // Initialise state machine
        let state_machine = Arc::new(Mutex::new(StateMachine::new(
            self.config.state_machine.clone(),
        )));

        let mut retry_gov = RetryGovernor::new();
        for agent in &self.config.agents {
            if let Some(retry) = &agent.retry {
                retry_gov.register(&agent.id, retry.max_attempts);
            }
        }
        let retry_governor = Arc::new(Mutex::new(retry_gov));

        let tools: Arc<HashMap<String, Arc<dyn Tool>>> =
            Arc::new(self.tools.clone());

        // Emit pipeline_started
        self.hub
            .emit(PipelineEvent {
                run_id: run_id.clone(),
                timestamp: Utc::now(),
                payload: EventPayload::PipelineStarted {
                    pipeline_name: self.config.name.clone(),
                },
            })
            .await;

        // Transition initialised -> running
        {
            let prev = {
                let mut sm = state_machine.lock().await;
                sm.transition("running")?
            };
            self.hub
                .emit(PipelineEvent {
                    run_id: run_id.clone(),
                    timestamp: Utc::now(),
                    payload: EventPayload::StateTransition {
                        from: prev,
                        to: "running".to_string(),
                    },
                })
                .await;
        }

        let dispatcher = Dispatcher {
            config: self.config.clone(),
            claude: self.claude.clone(),
            tools,
            hub: self.hub.clone(),
            state_machine: state_machine.clone(),
            retry_governor,
            run_id: run_id.clone(),
        };

        let result = dispatcher.run(initial_input).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(_) => {
                let prev = {
                    let mut sm = state_machine.lock().await;
                    sm.transition("complete")?
                };
                self.hub
                    .emit(PipelineEvent {
                        run_id: run_id.clone(),
                        timestamp: Utc::now(),
                        payload: EventPayload::StateTransition {
                            from: prev,
                            to: "complete".to_string(),
                        },
                    })
                    .await;
                self.hub
                    .emit(PipelineEvent {
                        run_id: run_id.clone(),
                        timestamp: Utc::now(),
                        payload: EventPayload::PipelineCompleted {
                            total_modules: 1,
                            duration_ms,
                        },
                    })
                    .await;
            }
            Err(ref e) => {
                let reason = e.to_string();
                {
                    let mut sm = state_machine.lock().await;
                    let prev = sm.transition("failed")?;
                    drop(sm);
                    self.hub
                        .emit(PipelineEvent {
                            run_id: run_id.clone(),
                            timestamp: Utc::now(),
                            payload: EventPayload::StateTransition {
                                from: prev,
                                to: "failed".to_string(),
                            },
                        })
                        .await;
                }
                self.hub
                    .emit(PipelineEvent {
                        run_id: run_id.clone(),
                        timestamp: Utc::now(),
                        payload: EventPayload::PipelineFailed {
                            reason: reason.clone(),
                        },
                    })
                    .await;
                // Brief pause so sinks and the UI can receive the final events
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                return Err(AgentFlowError::PipelineFailed(reason));
            }
        }

        // Brief pause so sinks and the UI can receive the final events
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        Ok(())
    }
}
