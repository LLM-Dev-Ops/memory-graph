//! Decision Memory Agent
//!
//! This agent persists decisions, outcomes, and reasoning artifacts for audit and learning.
//! It is classified as MEMORY_WRITE and operates strictly on structured memory data.
//!
//! # Classification
//! - **Type**: MEMORY_WRITE
//! - **decision_type**: decision_memory_capture
//!
//! # Contract
//! - Imports schemas exclusively from agentics-contracts
//! - Validates all inputs and outputs against contracts
//! - Emits telemetry compatible with LLM-Observatory
//! - Emits exactly ONE DecisionEvent to ruvector-service per invocation
//! - Exposes CLI-invokable endpoints (inspect / retrieve / replay)
//! - Deployable as a Google Cloud Edge Function
//! - Returns deterministic, machine-readable output
//!
//! # Non-Responsibilities (MUST NEVER)
//! - Modify system behavior
//! - Trigger remediation
//! - Trigger retries
//! - Emit alerts
//! - Enforce policies
//! - Perform orchestration
//! - Connect directly to Google SQL
//! - Execute SQL

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

pub mod contracts;
pub mod error;
pub mod ruvector;
pub mod agent;
pub mod telemetry;
pub mod handler;

// Re-exports
pub use contracts::*;
pub use error::{AgentError, AgentResult};
pub use ruvector::RuVectorClient;
pub use agent::DecisionMemoryAgent;
pub use handler::EdgeFunctionHandler;

/// Agent metadata constants
pub mod constants {
    /// Agent identifier
    pub const AGENT_ID: &str = "decision-memory-agent";
    /// Current version following semver
    pub const AGENT_VERSION: &str = "1.0.0";
    /// Decision type for this agent
    pub const DECISION_TYPE: &str = "decision_memory_capture";
    /// Agent classification
    pub const CLASSIFICATION: &str = "MEMORY_WRITE";
}
