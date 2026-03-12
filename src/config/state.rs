use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineConfig {
    pub initial: String,
    pub terminal: Vec<String>,
    pub states: Vec<StateConfig>,
}
