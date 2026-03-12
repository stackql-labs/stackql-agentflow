use super::traits::{AgentContext, QAIssue};

impl AgentContext {
    /// Build the user-facing message text, incorporating any QA feedback.
    pub fn build_user_message(&self) -> String {
        let mut parts = vec![self.user_input.clone()];

        if !self.feedback.is_empty() {
            let mut fb = format!(
                "\n\n---\n## Reviewer Feedback (attempt {})\n\nPlease address the following issues:\n",
                self.attempt
            );
            for issue in &self.feedback {
                fb.push_str(&format!(
                    "\n- [{}] {}\n  Suggestion: {}",
                    issue.severity.to_uppercase(),
                    issue.description,
                    issue.suggestion
                ));
            }
            fb.push_str("\n---\n");
            parts.push(fb);
        }

        parts.join("")
    }
}

/// Fluent builder for `AgentContext`.
pub struct AgentContextBuilder {
    agent_id: String,
    system_prompt: String,
    user_input: String,
    feedback: Vec<QAIssue>,
    attempt: u32,
    tool_ids: Vec<String>,
    model: String,
    max_tokens: u32,
}

impl AgentContextBuilder {
    pub fn new(agent_id: impl Into<String>, system_prompt: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            system_prompt: system_prompt.into(),
            user_input: String::new(),
            feedback: Vec::new(),
            attempt: 1,
            tool_ids: Vec::new(),
            model: "claude-sonnet-4-6".to_string(),
            max_tokens: 1024,
        }
    }

    pub fn user_input(mut self, input: impl Into<String>) -> Self {
        self.user_input = input.into();
        self
    }

    pub fn feedback(mut self, issues: Vec<QAIssue>) -> Self {
        self.feedback = issues;
        self
    }

    pub fn attempt(mut self, n: u32) -> Self {
        self.attempt = n;
        self
    }

    pub fn tools(mut self, ids: Vec<String>) -> Self {
        self.tool_ids = ids;
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn build(self) -> AgentContext {
        AgentContext {
            agent_id: self.agent_id,
            system_prompt: self.system_prompt,
            user_input: self.user_input,
            feedback: self.feedback,
            attempt: self.attempt,
            tool_ids: self.tool_ids,
            model: self.model,
            max_tokens: self.max_tokens,
        }
    }
}
