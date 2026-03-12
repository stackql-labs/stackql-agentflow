//! # stackql-agentflow
//!
//! A Rust-native multi-agent orchestration framework.
//! Pipelines are defined in YAML, agents are powered by Claude, state machines
//! are explicit and type-safe, and observability is built in.

pub mod agents;
pub mod claude;
pub mod config;
pub mod error;
pub mod observability;
pub mod orchestrator;
pub mod retry;
pub mod state;
pub mod tools;

mod pipeline;

pub use error::AgentFlowError;
pub use observability::{ConsoleSink, FileSink, LogSink, WebhookSink};
pub use pipeline::Pipeline;
pub use tools::FilesystemTool;
