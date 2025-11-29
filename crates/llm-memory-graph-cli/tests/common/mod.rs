//! Common test utilities for CLI integration tests

use llm_memory_graph::{engine::AsyncMemoryGraph, Config};
use llm_memory_graph_types::{AgentNode, PromptNode, ResponseNode, SessionId, TemplateNode, TokenUsage};
use std::collections::HashMap;
use std::path::Path;
use tempfile::TempDir;
use uuid::Uuid;

/// Test database wrapper with automatic cleanup
pub struct TestDb {
    pub tempdir: TempDir,
    pub graph: AsyncMemoryGraph,
}

impl TestDb {
    /// Create a new test database
    pub async fn new() -> Self {
        let tempdir = TempDir::new().expect("Failed to create temp dir");
        let config = Config::new(tempdir.path().to_str().unwrap());
        let graph = AsyncMemoryGraph::open(config)
            .await
            .expect("Failed to open database");

        Self { tempdir, graph }
    }

    /// Get the database path
    pub fn path(&self) -> &Path {
        self.tempdir.path()
    }

    /// Get path as string
    pub fn path_str(&self) -> &str {
        self.path().to_str().unwrap()
    }

    /// Create a test session with optional metadata
    pub async fn create_test_session(&self, metadata: Option<HashMap<String, String>>) -> SessionId {
        let session_id = SessionId::new();
        self.graph
            .create_session(session_id, metadata.unwrap_or_default())
            .await
            .expect("Failed to create session");
        session_id
    }

    /// Add a test prompt to a session
    pub async fn add_test_prompt(&self, session_id: SessionId, content: &str) -> PromptNode {
        self.graph
            .add_prompt(session_id, content.to_string(), None)
            .await
            .expect("Failed to add prompt")
    }

    /// Add a test response to a prompt
    pub async fn add_test_response(&self, prompt_id: Uuid, content: &str) -> ResponseNode {
        let token_usage = TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        };

        self.graph
            .add_response(prompt_id.into(), content.to_string(), Some(token_usage), None)
            .await
            .expect("Failed to add response")
    }

    /// Create a test agent
    pub async fn create_test_agent(&self, name: &str, model: Option<&str>) -> AgentNode {
        let metadata = HashMap::new();
        self.graph
            .create_agent(name.to_string(), model.map(String::from), metadata)
            .await
            .expect("Failed to create agent")
    }

    /// Create a test template
    pub async fn create_test_template(&self, name: &str, content: &str) -> TemplateNode {
        let metadata = HashMap::new();
        self.graph
            .create_template(name.to_string(), content.to_string(), metadata)
            .await
            .expect("Failed to create template")
    }

    /// Get database statistics
    pub async fn stats(&self) -> (usize, usize, usize) {
        let stats = self.graph.stats().await.expect("Failed to get stats");
        (stats.node_count, stats.edge_count, stats.session_count)
    }

    /// Flush database to disk
    pub async fn flush(&self) {
        self.graph.flush().await.expect("Failed to flush");
    }
}

/// Assert that output contains all expected strings
pub fn assert_output_contains(output: &str, expected: &[&str]) {
    for exp in expected {
        assert!(
            output.contains(exp),
            "Output should contain '{}'\nActual output:\n{}",
            exp,
            output
        );
    }
}

/// Assert that output is valid JSON
pub fn assert_valid_json(output: &str) -> serde_json::Value {
    serde_json::from_str(output).expect("Output should be valid JSON")
}

/// Assert that output is valid YAML
pub fn assert_valid_yaml(output: &str) -> serde_yaml::Value {
    serde_yaml::from_str(output).expect("Output should be valid YAML")
}

/// Create a sample export file for import testing
pub async fn create_sample_export(path: &Path, session_id: SessionId) -> std::io::Result<()> {
    let data = serde_json::json!({
        "session": {
            "id": session_id.to_string(),
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "metadata": {},
            "tags": []
        },
        "nodes": [],
        "edges": []
    });

    std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_test_db_creation() {
        let db = TestDb::new().await;
        let (nodes, edges, sessions) = db.stats().await;
        assert_eq!(nodes, 0);
        assert_eq!(edges, 0);
        assert_eq!(sessions, 0);
    }

    #[tokio::test]
    async fn test_create_session() {
        let db = TestDb::new().await;
        let session_id = db.create_test_session(None).await;
        let session = db.graph.get_session(session_id).await.unwrap();
        assert_eq!(session.id, session_id);
    }

    #[tokio::test]
    async fn test_add_prompt() {
        let db = TestDb::new().await;
        let session_id = db.create_test_session(None).await;
        let prompt = db.add_test_prompt(session_id, "Test prompt").await;
        assert_eq!(prompt.content, "Test prompt");
        assert_eq!(prompt.session_id, session_id);
    }

    #[tokio::test]
    async fn test_add_response() {
        let db = TestDb::new().await;
        let session_id = db.create_test_session(None).await;
        let prompt = db.add_test_prompt(session_id, "Test prompt").await;
        let response = db.add_test_response(prompt.id.into(), "Test response").await;
        assert_eq!(response.content, "Test response");
    }
}
