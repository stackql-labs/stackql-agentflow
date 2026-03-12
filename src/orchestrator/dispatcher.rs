use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use chrono::Utc;
use tokio::sync::Mutex;

use crate::{
    agents::{
        context::AgentContextBuilder,
        runner::ClaudeAgentRunner,
        traits::{AgentOutput, QAIssue},
    },
    claude::ClaudeClient,
    config::{agent::AgentConfig, pipeline::PipelineConfig},
    error::AgentFlowError,
    observability::{
        event::{EventPayload, PipelineEvent},
        hub::EventHub,
    },
    retry::RetryGovernor,
    state::StateMachine,
    tools::traits::Tool,
};

/// Executes agents according to the pipeline's dispatch configuration and
/// manages the QA feedback / retry loop.
pub struct Dispatcher {
    pub config: PipelineConfig,
    /// Base directory for resolving relative prompt paths declared in the YAML.
    pub base_dir: PathBuf,
    pub claude: Arc<ClaudeClient>,
    pub tools: Arc<HashMap<String, Arc<dyn Tool>>>,
    pub hub: Arc<EventHub>,
    pub state_machine: Arc<Mutex<StateMachine>>,
    pub retry_governor: Arc<Mutex<RetryGovernor>>,
    pub run_id: String,
}

impl Dispatcher {
    /// Run the full pipeline starting from the first agent.
    pub async fn run(&self, initial_input: &str) -> Result<AgentOutput, AgentFlowError> {
        let first_id = self.find_first_agent().ok_or_else(|| {
            AgentFlowError::Config("pipeline has no agents".to_string())
        })?;
        self.run_chain(&first_id, initial_input.to_string()).await
    }

    // -----------------------------------------------------------------------
    // Private chain runner
    // -----------------------------------------------------------------------

    /// Run agent `agent_id` and follow its dispatch chain.
    async fn run_chain(
        &self,
        agent_id: &str,
        input: String,
    ) -> Result<AgentOutput, AgentFlowError> {
        let output = self.run_agent(agent_id, input.clone(), vec![], 1).await?;

        let agent_cfg = self.get_agent(agent_id)?;

        // Apply transitions
        self.apply_transitions(&agent_cfg, output.passed).await;

        // Follow dispatch config
        if let Some(dispatch) = &agent_cfg.dispatch {
            match dispatch.strategy.as_str() {
                "sequential" => {
                    if let Some(target_id) = &dispatch.target {
                        return self
                            .run_sequential_qa_loop(
                                agent_id,
                                target_id,
                                &input,
                                output.content,
                            )
                            .await;
                    }
                }
                "parallel_fan_out" => {
                    if let Some(target_id) = &dispatch.target {
                        return self
                            .run_fan_out(target_id, &output.content)
                            .await;
                    }
                }
                "parallel_broadcast" => {
                    if let Some(targets) = &dispatch.targets {
                        return self
                            .run_broadcast(targets, &output.content)
                            .await;
                    }
                }
                unknown => {
                    tracing::warn!("unknown dispatch strategy: {}", unknown);
                }
            }
        }

        Ok(output)
    }

    // -----------------------------------------------------------------------
    // Single-agent runner (no chain following)
    // -----------------------------------------------------------------------

    async fn run_agent(
        &self,
        agent_id: &str,
        input: String,
        feedback: Vec<QAIssue>,
        attempt: u32,
    ) -> Result<AgentOutput, AgentFlowError> {
        let agent_cfg = self.get_agent(agent_id)?;

        let model = agent_cfg
            .model
            .clone()
            .or_else(|| self.config.defaults.model.clone())
            .unwrap_or_else(|| "claude-sonnet-4-6".to_string());
        let max_tokens = agent_cfg
            .max_tokens
            .or(self.config.defaults.max_tokens)
            .unwrap_or(1024);

        let prompt_path = self.base_dir.join(&agent_cfg.prompt);
        let prompt_text = std::fs::read_to_string(&prompt_path).map_err(|e| {
            AgentFlowError::Config(format!(
                "failed to read prompt '{}': {}",
                prompt_path.display(),
                e
            ))
        })?;

        // Apply on_start transition
        if let Some(state) = &agent_cfg.transitions.on_start {
            let mut sm = self.state_machine.lock().await;
            let _ = sm.transition(state);
        }

        let ctx = AgentContextBuilder::new(agent_id, &prompt_text)
            .user_input(&input)
            .feedback(feedback)
            .attempt(attempt)
            .tools(agent_cfg.tools.clone())
            .model(model)
            .max_tokens(max_tokens)
            .build();

        let runner = ClaudeAgentRunner::new(
            self.claude.clone(),
            self.tools.clone(),
            self.hub.clone(),
            self.run_id.clone(),
        );

        runner.run_with_id(&ctx, agent_id).await
    }

    // -----------------------------------------------------------------------
    // Sequential QA feedback / retry loop
    // -----------------------------------------------------------------------

    async fn run_sequential_qa_loop(
        &self,
        producer_id: &str,
        qa_id: &str,
        original_input: &str,
        initial_producer_content: String,
    ) -> Result<AgentOutput, AgentFlowError> {
        let qa_cfg = self.get_agent(qa_id)?;
        let max_attempts = qa_cfg
            .retry
            .as_ref()
            .map(|r| r.max_attempts)
            .unwrap_or(1);
        let backoff_ms = qa_cfg
            .retry
            .as_ref()
            .and_then(|r| r.backoff_ms)
            .unwrap_or(0);

        let mut current_content = initial_producer_content;
        let mut producer_attempt: u32 = 1;

        for qa_attempt in 1..=max_attempts {
            let qa_output = self
                .run_agent(qa_id, current_content.clone(), vec![], qa_attempt)
                .await?;

            // Apply QA transitions
            self.apply_transitions(&qa_cfg, qa_output.passed).await;

            if qa_output.passed {
                return Ok(qa_output);
            }

            // QA failed — decide whether to retry
            let is_last_attempt = qa_attempt == max_attempts;
            if is_last_attempt {
                return Err(AgentFlowError::MaxRetriesExceeded {
                    agent_id: qa_id.to_string(),
                });
            }

            // Emit feedback_issued
            self.hub
                .emit(PipelineEvent {
                    run_id: self.run_id.clone(),
                    timestamp: Utc::now(),
                    payload: EventPayload::FeedbackIssued {
                        qa_agent_id: qa_id.to_string(),
                        target_agent_id: producer_id.to_string(),
                        issue_count: qa_output.issues.len(),
                    },
                })
                .await;

            let feedback = qa_output.issues;

            if backoff_ms > 0 {
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }

            producer_attempt += 1;
            let producer_output = self
                .run_agent(
                    producer_id,
                    original_input.to_string(),
                    feedback,
                    producer_attempt,
                )
                .await?;

            current_content = producer_output.content;
        }

        Err(AgentFlowError::MaxRetriesExceeded {
            agent_id: qa_id.to_string(),
        })
    }

    // -----------------------------------------------------------------------
    // Parallel strategies
    // -----------------------------------------------------------------------
    // NOTE: parallel_fan_out and parallel_broadcast execute sequentially here.
    // Full tokio::task::JoinSet parallelism requires wrapping Dispatcher in
    // Arc and boxing the recursive future — tracked on the roadmap.

    /// Fan-out: each non-empty line of `content` becomes a work item.
    /// Runs target agents sequentially (parallel execution is on the roadmap).
    async fn run_fan_out(
        &self,
        target_id: &str,
        content: &str,
    ) -> Result<AgentOutput, AgentFlowError> {
        let items: Vec<String> = content
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

        if items.is_empty() {
            return Ok(AgentOutput {
                passed: true,
                content: String::new(),
                issues: vec![],
            });
        }

        let mut all_passed = true;
        let mut last_content = String::new();
        for item in items {
            let out = self.run_agent(target_id, item, vec![], 1).await?;
            if !out.passed {
                all_passed = false;
            }
            last_content = out.content;
        }

        Ok(AgentOutput {
            passed: all_passed,
            content: last_content,
            issues: vec![],
        })
    }

    /// Broadcast: run all `target_ids` with the same input.
    /// Runs sequentially (parallel execution is on the roadmap).
    async fn run_broadcast(
        &self,
        target_ids: &[String],
        content: &str,
    ) -> Result<AgentOutput, AgentFlowError> {
        let mut all_passed = true;
        for tid in target_ids {
            let out = self.run_agent(tid, content.to_string(), vec![], 1).await?;
            if !out.passed {
                all_passed = false;
            }
        }

        Ok(AgentOutput {
            passed: all_passed,
            content: String::new(),
            issues: vec![],
        })
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn get_agent(&self, id: &str) -> Result<AgentConfig, AgentFlowError> {
        self.config
            .agents
            .iter()
            .find(|a| a.id == id)
            .cloned()
            .ok_or_else(|| AgentFlowError::Config(format!("agent '{}' not found", id)))
    }

    /// The first agent in the chain is the one not referenced as a dispatch target.
    fn find_first_agent(&self) -> Option<String> {
        let targets: std::collections::HashSet<String> = self
            .config
            .agents
            .iter()
            .flat_map(|a| {
                let mut t = vec![];
                if let Some(d) = &a.dispatch {
                    if let Some(tgt) = &d.target {
                        t.push(tgt.clone());
                    }
                    if let Some(tgts) = &d.targets {
                        t.extend(tgts.clone());
                    }
                }
                t
            })
            .collect();

        self.config
            .agents
            .iter()
            .find(|a| !targets.contains(&a.id))
            .map(|a| a.id.clone())
    }

    async fn apply_transitions(&self, agent_cfg: &AgentConfig, passed: bool) {
        let mut sm = self.state_machine.lock().await;
        if passed {
            if let Some(state) = &agent_cfg.transitions.on_pass {
                let _ = sm.transition(state);
            } else if let Some(state) = &agent_cfg.transitions.on_complete {
                let _ = sm.transition(state);
            }
        } else if let Some(state) = &agent_cfg.transitions.on_fail {
            let _ = sm.transition(state);
        }
    }
}
