//! Example metadata enrichment plugin for LLM-Memory-Graph
//!
//! This plugin demonstrates how to implement enrichment hooks that automatically
//! enhance nodes and sessions with additional metadata, analytics, and derived information.
//!
//! # Features
//!
//! - Automatic timestamp enrichment
//! - Content analysis (word count, character count, etc.)
//! - Session statistics tracking
//! - Custom metadata injection
//! - Event correlation IDs
//!
//! # Usage
//!
//! ```rust
//! use example_enricher::EnrichmentPlugin;
//! use llm_memory_graph::plugin::PluginManager;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut manager = PluginManager::new();
//! let enricher = Arc::new(EnrichmentPlugin::new());
//! manager.register(enricher).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use chrono::Utc;
use llm_memory_graph::plugin::{Plugin, PluginBuilder, PluginContext, PluginError, PluginMetadata};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, info};

/// Enrichment statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentStats {
    /// Number of nodes enriched
    pub nodes_enriched: u64,
    /// Number of sessions enriched
    pub sessions_enriched: u64,
    /// Total words analyzed
    pub total_words_analyzed: u64,
    /// Total characters analyzed
    pub total_characters_analyzed: u64,
}

/// Content analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAnalysis {
    /// Word count
    pub word_count: usize,
    /// Character count
    pub character_count: usize,
    /// Line count
    pub line_count: usize,
    /// Average word length
    pub avg_word_length: f64,
    /// Sentence count (estimated)
    pub sentence_count: usize,
}

impl ContentAnalysis {
    /// Analyze content
    fn analyze(content: &str) -> Self {
        let character_count = content.len();
        let line_count = content.lines().count();

        let words: Vec<&str> = content.split_whitespace().collect();
        let word_count = words.len();

        let avg_word_length = if word_count > 0 {
            words.iter().map(|w| w.len()).sum::<usize>() as f64 / word_count as f64
        } else {
            0.0
        };

        // Estimate sentence count by counting sentence-ending punctuation
        let sentence_count = content
            .chars()
            .filter(|c| matches!(c, '.' | '!' | '?'))
            .count()
            .max(1);

        Self {
            word_count,
            character_count,
            line_count,
            avg_word_length,
            sentence_count,
        }
    }

    /// Convert to JSON metadata
    fn to_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("word_count".to_string(), self.word_count.to_string());
        metadata.insert(
            "character_count".to_string(),
            self.character_count.to_string(),
        );
        metadata.insert("line_count".to_string(), self.line_count.to_string());
        metadata.insert(
            "avg_word_length".to_string(),
            format!("{:.2}", self.avg_word_length),
        );
        metadata.insert(
            "sentence_count".to_string(),
            self.sentence_count.to_string(),
        );
        metadata
    }
}

/// Metadata enrichment plugin
///
/// Automatically enriches nodes and sessions with additional metadata,
/// analytics, and derived information.
pub struct EnrichmentPlugin {
    metadata: PluginMetadata,
    stats: Arc<EnrichmentStatsInner>,
}

struct EnrichmentStatsInner {
    nodes_enriched: AtomicU64,
    sessions_enriched: AtomicU64,
    total_words: AtomicU64,
    total_characters: AtomicU64,
}

impl EnrichmentPlugin {
    /// Create a new enrichment plugin
    pub fn new() -> Self {
        let metadata = PluginBuilder::new("metadata_enricher", "1.0.0")
            .author("LLM DevOps Team")
            .description("Enriches nodes and sessions with additional metadata and analytics")
            .capability("enrichment")
            .capability("metadata")
            .capability("analytics")
            .build();

        let stats = Arc::new(EnrichmentStatsInner {
            nodes_enriched: AtomicU64::new(0),
            sessions_enriched: AtomicU64::new(0),
            total_words: AtomicU64::new(0),
            total_characters: AtomicU64::new(0),
        });

        Self { metadata, stats }
    }

    /// Get enrichment statistics
    pub fn get_stats(&self) -> EnrichmentStats {
        EnrichmentStats {
            nodes_enriched: self.stats.nodes_enriched.load(Ordering::Relaxed),
            sessions_enriched: self.stats.sessions_enriched.load(Ordering::Relaxed),
            total_words_analyzed: self.stats.total_words.load(Ordering::Relaxed),
            total_characters_analyzed: self.stats.total_characters.load(Ordering::Relaxed),
        }
    }

    /// Extract content from context
    fn extract_content(&self, context: &PluginContext) -> Option<String> {
        context
            .data()
            .get("content")
            .or_else(|| context.data().get("text"))
            .or_else(|| context.data().get("body"))
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    /// Enrich with content analysis
    fn enrich_content_analysis(&self, content: &str) -> HashMap<String, String> {
        let analysis = ContentAnalysis::analyze(content);

        // Update stats
        self.stats
            .total_words
            .fetch_add(analysis.word_count as u64, Ordering::Relaxed);
        self.stats
            .total_characters
            .fetch_add(analysis.character_count as u64, Ordering::Relaxed);

        analysis.to_metadata()
    }

    /// Generate correlation ID
    fn generate_correlation_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

impl Default for EnrichmentPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for EnrichmentPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(&self) -> Result<(), PluginError> {
        info!("EnrichmentPlugin initialized");
        Ok(())
    }

    async fn after_create_node(&self, context: &PluginContext) -> Result<(), PluginError> {
        debug!("EnrichmentPlugin: after_create_node hook");

        // Log enrichment details
        let enrichment_time = Utc::now();
        let correlation_id = Self::generate_correlation_id();

        info!(
            "Node created at {} (correlation_id: {}) for operation: {}",
            enrichment_time, correlation_id, context.operation
        );

        // Analyze content if present
        if let Some(content) = self.extract_content(context) {
            let content_metadata = self.enrich_content_analysis(&content);

            debug!(
                "Content analysis: {} words, {} characters",
                content_metadata.get("word_count").unwrap_or(&"0".to_string()),
                content_metadata
                    .get("character_count")
                    .unwrap_or(&"0".to_string())
            );
        }

        // Update stats
        self.stats.nodes_enriched.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    async fn after_create_session(&self, context: &PluginContext) -> Result<(), PluginError> {
        debug!("EnrichmentPlugin: after_create_session hook");

        let session_id = context
            .data()
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        info!(
            "Session created: {} at {}",
            session_id,
            Utc::now().to_rfc3339()
        );

        // Log session metadata
        if let Some(metadata) = context.data().get("metadata") {
            if let Some(metadata_obj) = metadata.as_object() {
                debug!("Session metadata fields: {:?}", metadata_obj.keys());
            }
        }

        // Update stats
        self.stats
            .sessions_enriched
            .fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    async fn after_query(&self, context: &PluginContext) -> Result<(), PluginError> {
        debug!("EnrichmentPlugin: after_query hook");

        // Log query execution
        let query_time = Utc::now();
        let correlation_id = Self::generate_correlation_id();

        if let Some(results) = context.data().get("results") {
            if let Some(results_array) = results.as_array() {
                info!(
                    "Query completed at {} (correlation_id: {}): {} results",
                    query_time,
                    correlation_id,
                    results_array.len()
                );
            }
        }

        Ok(())
    }

    async fn after_create_edge(&self, context: &PluginContext) -> Result<(), PluginError> {
        debug!("EnrichmentPlugin: after_create_edge hook");

        // Log edge creation
        let edge_type = context
            .data()
            .get("edge_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let from_node = context
            .data()
            .get("from_node_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let to_node = context
            .data()
            .get("to_node_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        info!(
            "Edge created: {} -> {} (type: {})",
            from_node, to_node, edge_type
        );

        Ok(())
    }
}

/// Enrichment builder
///
/// Provides a way to configure enrichment behavior.
pub struct EnrichmentBuilder {
    // Future: Add configuration options
}

impl EnrichmentBuilder {
    /// Create a new enrichment builder
    pub fn new() -> Self {
        Self {}
    }

    /// Build the enrichment plugin
    pub fn build(self) -> EnrichmentPlugin {
        EnrichmentPlugin::new()
    }
}

impl Default for EnrichmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_content_analysis() {
        let content = "This is a test. It has multiple sentences! How many words?";
        let analysis = ContentAnalysis::analyze(content);

        assert!(analysis.word_count > 0);
        assert!(analysis.sentence_count >= 3);
        assert!(analysis.character_count > 0);
    }

    #[tokio::test]
    async fn test_enrichment_plugin() {
        let plugin = EnrichmentPlugin::new();
        let context = PluginContext::new(
            "test",
            json!({
                "content": "This is test content with multiple words."
            }),
        );

        assert!(plugin.after_create_node(&context).await.is_ok());

        let stats = plugin.get_stats();
        assert_eq!(stats.nodes_enriched, 1);
        assert!(stats.total_words_analyzed > 0);
    }

    #[tokio::test]
    async fn test_session_enrichment() {
        let plugin = EnrichmentPlugin::new();
        let context = PluginContext::new(
            "test",
            json!({
                "session_id": "test-session-123",
                "metadata": {
                    "user": "test_user",
                    "environment": "test"
                }
            }),
        );

        assert!(plugin.after_create_session(&context).await.is_ok());

        let stats = plugin.get_stats();
        assert_eq!(stats.sessions_enriched, 1);
    }
}
