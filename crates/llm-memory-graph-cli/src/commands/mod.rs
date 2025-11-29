//! CLI command implementations
//!
//! This module contains all the command implementations for the CLI tool.
//! Each command is organized into its own submodule for better maintainability.

pub mod agent;
pub mod export;
pub mod import;
pub mod query;
pub mod server;
pub mod session;
pub mod stats;
pub mod template;

use llm_memory_graph::engine::AsyncMemoryGraph;

use crate::output::OutputFormat;

/// Common context passed to all commands
pub struct CommandContext<'a> {
    pub graph: &'a AsyncMemoryGraph,
    pub format: &'a OutputFormat,
}

impl<'a> CommandContext<'a> {
    pub fn new(graph: &'a AsyncMemoryGraph, format: &'a OutputFormat) -> Self {
        Self { graph, format }
    }
}
