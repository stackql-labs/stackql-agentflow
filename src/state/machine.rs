use crate::{config::StateMachineConfig, error::AgentFlowError};

/// Runtime state machine.  Validates transitions against the declared states
/// and rejects attempts to leave a terminal state.
pub struct StateMachine {
    config: StateMachineConfig,
    current: String,
}

impl StateMachine {
    pub fn new(config: StateMachineConfig) -> Self {
        let initial = config.initial.clone();
        Self {
            config,
            current: initial,
        }
    }

    pub fn current(&self) -> &str {
        &self.current
    }

    pub fn is_terminal(&self) -> bool {
        self.config.terminal.contains(&self.current)
    }

    /// Transition to `next_state`.  Returns the previous state on success.
    pub fn transition(&mut self, next_state: &str) -> Result<String, AgentFlowError> {
        if self.is_terminal() {
            return Err(AgentFlowError::State(format!(
                "cannot leave terminal state '{}'",
                self.current
            )));
        }
        let valid = self
            .config
            .states
            .iter()
            .any(|s| s.id == next_state);
        if !valid {
            return Err(AgentFlowError::State(format!(
                "unknown state '{}'",
                next_state
            )));
        }
        let prev = std::mem::replace(&mut self.current, next_state.to_string());
        Ok(prev)
    }
}
