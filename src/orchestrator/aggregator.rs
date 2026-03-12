use crate::config::pipeline::AggregationConfig;
use crate::agents::traits::AgentOutput;

/// Decision returned by the aggregator.
#[derive(Debug)]
pub enum AggregationResult {
    Complete,
    Failed,
}

/// Evaluates gate outcomes against an `AggregationConfig`.
pub struct Aggregator {
    config: AggregationConfig,
}

impl Aggregator {
    pub fn new(config: AggregationConfig) -> Self {
        Self { config }
    }

    /// Evaluate a set of gate outputs.
    /// `results` maps gate agent ID -> AgentOutput.
    pub fn evaluate(&self, results: &[(String, AgentOutput)]) -> AggregationResult {
        let gate_results: Vec<bool> = self
            .config
            .gates
            .iter()
            .map(|gate_id| {
                results
                    .iter()
                    .find(|(id, _)| id == gate_id)
                    .map(|(_, out)| out.passed)
                    .unwrap_or(false)
            })
            .collect();

        match self.config.strategy.as_str() {
            "all_pass" => {
                if gate_results.iter().all(|&p| p) {
                    AggregationResult::Complete
                } else {
                    AggregationResult::Failed
                }
            }
            "any_pass" => {
                if gate_results.iter().any(|&p| p) {
                    AggregationResult::Complete
                } else {
                    AggregationResult::Failed
                }
            }
            "threshold" => {
                let min = self.config.min_pass.unwrap_or(1);
                let passed = gate_results.iter().filter(|&&p| p).count();
                if passed >= min {
                    AggregationResult::Complete
                } else {
                    AggregationResult::Failed
                }
            }
            _ => AggregationResult::Failed,
        }
    }
}
