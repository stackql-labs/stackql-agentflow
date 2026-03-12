use crate::config::agent::TransitionConfig;

use super::machine::StateMachine;
use crate::error::AgentFlowError;

/// Apply the `on_start` transition if defined.
pub fn apply_on_start(
    sm: &mut StateMachine,
    tc: &TransitionConfig,
) -> Result<Option<String>, AgentFlowError> {
    if let Some(state) = &tc.on_start {
        let prev = sm.transition(state)?;
        return Ok(Some(prev));
    }
    Ok(None)
}

/// Apply the `on_complete` transition if defined.
pub fn apply_on_complete(
    sm: &mut StateMachine,
    tc: &TransitionConfig,
) -> Result<Option<String>, AgentFlowError> {
    if let Some(state) = &tc.on_complete {
        let prev = sm.transition(state)?;
        return Ok(Some(prev));
    }
    Ok(None)
}

/// Apply the `on_pass` transition if defined.
pub fn apply_on_pass(
    sm: &mut StateMachine,
    tc: &TransitionConfig,
) -> Result<Option<String>, AgentFlowError> {
    if let Some(state) = &tc.on_pass {
        let prev = sm.transition(state)?;
        return Ok(Some(prev));
    }
    Ok(None)
}

/// Apply the `on_fail` transition if defined.
pub fn apply_on_fail(
    sm: &mut StateMachine,
    tc: &TransitionConfig,
) -> Result<Option<String>, AgentFlowError> {
    if let Some(state) = &tc.on_fail {
        let prev = sm.transition(state)?;
        return Ok(Some(prev));
    }
    Ok(None)
}
