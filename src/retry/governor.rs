use std::collections::HashMap;

/// Tracks per-agent attempt counts and decides whether another retry is allowed.
pub struct RetryGovernor {
    /// Number of attempts completed so far, keyed by agent ID.
    attempts: HashMap<String, u32>,
    /// Maximum allowed attempts per agent.
    max_attempts: HashMap<String, u32>,
}

impl RetryGovernor {
    pub fn new() -> Self {
        Self {
            attempts: HashMap::new(),
            max_attempts: HashMap::new(),
        }
    }

    /// Register the maximum number of attempts allowed for an agent.
    pub fn register(&mut self, agent_id: &str, max_attempts: u32) {
        self.max_attempts
            .insert(agent_id.to_string(), max_attempts);
    }

    /// Record that one attempt has completed for `agent_id`.
    /// Returns `true` if another retry is allowed.
    pub fn record_attempt(&mut self, agent_id: &str) -> bool {
        let count = self.attempts.entry(agent_id.to_string()).or_insert(0);
        *count += 1;
        let max = self.max_attempts.get(agent_id).copied().unwrap_or(1);
        *count < max
    }

    /// How many attempts have been recorded for `agent_id`.
    pub fn attempt_count(&self, agent_id: &str) -> u32 {
        self.attempts.get(agent_id).copied().unwrap_or(0)
    }

    pub fn reset(&mut self, agent_id: &str) {
        self.attempts.remove(agent_id);
    }
}

impl Default for RetryGovernor {
    fn default() -> Self {
        Self::new()
    }
}
