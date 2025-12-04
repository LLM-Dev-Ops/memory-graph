//! Mapping utilities for converting Observatory events to graph entities
//!
//! This module provides the SpanMapper and related utilities for bidirectional
//! mapping between Observatory telemetry data and memory graph entities.

use super::consumer::{SpanStatus, TelemetryData, TraceContext};
use super::events::MemoryGraphEvent;
use super::lineage::{LineageChain, LineageEdgeType, LineageNode};
use crate::{EdgeType, NodeId, NodeType, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mapping configuration for span-to-graph conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingConfig {
    /// Operation name patterns that map to specific node types
    pub operation_patterns: HashMap<String, NodeType>,
    /// Attribute keys that should be extracted as metadata
    pub metadata_keys: Vec<String>,
    /// Whether to create edges for all span relationships
    pub create_lineage_edges: bool,
    /// Whether to map trace IDs to session IDs
    pub trace_to_session: bool,
}

impl Default for MappingConfig {
    fn default() -> Self {
        let mut patterns = HashMap::new();
        patterns.insert("llm.generate".to_string(), NodeType::Prompt);
        patterns.insert("llm.completion".to_string(), NodeType::Response);
        patterns.insert("llm.prompt".to_string(), NodeType::Prompt);
        patterns.insert("llm.response".to_string(), NodeType::Response);
        patterns.insert("tool.".to_string(), NodeType::Tool);
        patterns.insert("function.".to_string(), NodeType::Tool);
        patterns.insert("agent.".to_string(), NodeType::Agent);

        Self {
            operation_patterns: patterns,
            metadata_keys: vec![
                "model".to_string(),
                "temperature".to_string(),
                "max_tokens".to_string(),
                "user_id".to_string(),
            ],
            create_lineage_edges: true,
            trace_to_session: true,
        }
    }
}

/// A mapped graph entity from telemetry data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedEntity {
    /// Graph node ID
    pub node_id: NodeId,
    /// Node type
    pub node_type: NodeType,
    /// Session ID (if applicable)
    pub session_id: Option<SessionId>,
    /// Source span ID
    pub source_span_id: String,
    /// Source trace ID
    pub source_trace_id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Extracted metadata
    pub metadata: HashMap<String, String>,
}

/// A mapped edge relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedEdge {
    /// From node ID
    pub from: NodeId,
    /// To node ID
    pub to: NodeId,
    /// Edge type
    pub edge_type: EdgeType,
    /// Source lineage edge type
    pub source_lineage_type: LineageEdgeType,
}

/// Result of a mapping operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingResult {
    /// Mapped entities (nodes)
    pub entities: Vec<MappedEntity>,
    /// Mapped edges
    pub edges: Vec<MappedEdge>,
    /// Any errors encountered during mapping
    pub errors: Vec<String>,
}

impl MappingResult {
    /// Create a new empty mapping result
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            edges: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add an entity
    pub fn add_entity(&mut self, entity: MappedEntity) {
        self.entities.push(entity);
    }

    /// Add an edge
    pub fn add_edge(&mut self, edge: MappedEdge) {
        self.edges.push(edge);
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Check if mapping was successful (no errors)
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for MappingResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Mapper for converting Observatory spans to memory graph entities
pub struct SpanMapper {
    /// Mapping configuration
    config: MappingConfig,
    /// Cache of span ID to node ID mappings
    span_to_node: HashMap<String, NodeId>,
}

impl SpanMapper {
    /// Create a new span mapper with default configuration
    pub fn new() -> Self {
        Self::with_config(MappingConfig::default())
    }

    /// Create a new span mapper with custom configuration
    pub fn with_config(config: MappingConfig) -> Self {
        Self {
            config,
            span_to_node: HashMap::new(),
        }
    }

    /// Map a single span to a graph entity
    pub fn map_span(&mut self, data: &TelemetryData) -> Option<MappedEntity> {
        if let TelemetryData::Span {
            span_id,
            trace_id,
            operation_name,
            start_time,
            attributes,
            ..
        } = data
        {
            // Infer node type from operation name
            let node_type = self.infer_node_type(operation_name);

            // Generate or retrieve node ID
            let node_id = self
                .span_to_node
                .entry(span_id.clone())
                .or_insert_with(NodeId::new)
                .clone();

            // Extract session ID if trace-to-session mapping is enabled
            let session_id = if self.config.trace_to_session {
                Some(SessionId::from_string(trace_id))
            } else {
                None
            };

            // Extract metadata
            let mut metadata = HashMap::new();
            for key in &self.config.metadata_keys {
                if let Some(value) = attributes.get(key) {
                    metadata.insert(key.clone(), value.clone());
                }
            }

            // Add span and trace IDs to metadata
            metadata.insert("span_id".to_string(), span_id.clone());
            metadata.insert("trace_id".to_string(), trace_id.clone());
            metadata.insert("operation".to_string(), operation_name.clone());

            Some(MappedEntity {
                node_id,
                node_type,
                session_id,
                source_span_id: span_id.clone(),
                source_trace_id: trace_id.clone(),
                timestamp: *start_time,
                metadata,
            })
        } else {
            None
        }
    }

    /// Map a lineage chain to graph entities and edges
    pub fn map_lineage_chain(&mut self, chain: &LineageChain) -> MappingResult {
        let mut result = MappingResult::new();

        // Map all nodes
        for node in &chain.nodes {
            if let Some(entity) = self.map_lineage_node(node, &chain.trace_id) {
                result.add_entity(entity);
            } else {
                result.add_error(format!("Failed to map lineage node: {}", node.id));
            }
        }

        // Map all edges if configured
        if self.config.create_lineage_edges {
            for edge in &chain.edges {
                if let Some(mapped_edge) = self.map_lineage_edge(edge) {
                    result.add_edge(mapped_edge);
                } else {
                    result.add_error(format!(
                        "Failed to map lineage edge: {} -> {}",
                        edge.from, edge.to
                    ));
                }
            }
        }

        result
    }

    /// Map a lineage node to a graph entity
    fn map_lineage_node(&mut self, node: &LineageNode, trace_id: &str) -> Option<MappedEntity> {
        let node_type = self.infer_node_type(&node.operation);

        let node_id = self
            .span_to_node
            .entry(node.id.clone())
            .or_insert_with(NodeId::new)
            .clone();

        let session_id = if self.config.trace_to_session {
            Some(SessionId::from_string(trace_id))
        } else {
            None
        };

        let mut metadata = node.attributes.clone();
        metadata.insert("span_id".to_string(), node.id.clone());
        metadata.insert("trace_id".to_string(), trace_id.to_string());
        metadata.insert("operation".to_string(), node.operation.clone());
        metadata.insert("duration_ms".to_string(), node.duration_ms.to_string());
        metadata.insert("status".to_string(), format!("{:?}", node.status));

        Some(MappedEntity {
            node_id,
            node_type,
            session_id,
            source_span_id: node.id.clone(),
            source_trace_id: trace_id.to_string(),
            timestamp: node.start_time,
            metadata,
        })
    }

    /// Map a lineage edge to a graph edge
    fn map_lineage_edge(&self, edge: &super::lineage::LineageEdge) -> Option<MappedEdge> {
        let from = self.span_to_node.get(&edge.from)?.clone();
        let to = self.span_to_node.get(&edge.to)?.clone();

        let edge_type = match edge.edge_type {
            LineageEdgeType::ParentChild => EdgeType::ParentChild,
            LineageEdgeType::Follows => EdgeType::Follows,
            LineageEdgeType::CausedBy => EdgeType::References,
            LineageEdgeType::DataFlow => EdgeType::Contains,
        };

        Some(MappedEdge {
            from,
            to,
            edge_type,
            source_lineage_type: edge.edge_type,
        })
    }

    /// Infer node type from operation name
    fn infer_node_type(&self, operation: &str) -> NodeType {
        // Check configured patterns
        for (pattern, node_type) in &self.config.operation_patterns {
            if operation.contains(pattern) {
                return *node_type;
            }
        }

        // Default fallback
        NodeType::Context
    }

    /// Convert a graph event to telemetry data
    pub fn event_to_telemetry(&self, event: &MemoryGraphEvent) -> Option<TelemetryData> {
        match event {
            MemoryGraphEvent::NodeCreated {
                node_id,
                node_type,
                session_id,
                timestamp,
                metadata,
            } => {
                let span_id = format!("node-{}", node_id);
                let trace_id = session_id
                    .as_ref()
                    .map(|s| format!("session-{}", s))
                    .unwrap_or_else(|| format!("trace-{}", node_id));

                Some(TelemetryData::Span {
                    span_id,
                    trace_id,
                    parent_span_id: None,
                    operation_name: format!("{:?}.created", node_type),
                    start_time: *timestamp,
                    end_time: *timestamp,
                    attributes: metadata.clone(),
                    status: SpanStatus::Ok,
                })
            }

            MemoryGraphEvent::PromptSubmitted {
                prompt_id,
                session_id,
                content_length,
                model,
                timestamp,
            } => {
                let span_id = format!("prompt-{}", prompt_id);
                let trace_id = format!("session-{}", session_id);

                let mut attributes = HashMap::new();
                attributes.insert("model".to_string(), model.clone());
                attributes.insert("content_length".to_string(), content_length.to_string());

                Some(TelemetryData::Span {
                    span_id,
                    trace_id,
                    parent_span_id: None,
                    operation_name: "llm.prompt.submit".to_string(),
                    start_time: *timestamp,
                    end_time: *timestamp,
                    attributes,
                    status: SpanStatus::Ok,
                })
            }

            MemoryGraphEvent::ResponseGenerated {
                response_id,
                prompt_id,
                tokens_used,
                latency_ms,
                timestamp,
                ..
            } => {
                let span_id = format!("response-{}", response_id);
                let trace_id = format!("prompt-{}", prompt_id);
                let parent_span_id = Some(format!("prompt-{}", prompt_id));

                let mut attributes = HashMap::new();
                attributes.insert(
                    "prompt_tokens".to_string(),
                    tokens_used.prompt_tokens.to_string(),
                );
                attributes.insert(
                    "completion_tokens".to_string(),
                    tokens_used.completion_tokens.to_string(),
                );

                let start_time = *timestamp - chrono::Duration::milliseconds(*latency_ms as i64);

                Some(TelemetryData::Span {
                    span_id,
                    trace_id,
                    parent_span_id,
                    operation_name: "llm.response.generate".to_string(),
                    start_time,
                    end_time: *timestamp,
                    attributes,
                    status: SpanStatus::Ok,
                })
            }

            _ => None,
        }
    }

    /// Get the node ID for a given span ID
    pub fn get_node_id(&self, span_id: &str) -> Option<NodeId> {
        self.span_to_node.get(span_id).cloned()
    }

    /// Clear the mapping cache
    pub fn clear_cache(&mut self) {
        self.span_to_node.clear();
    }
}

impl Default for SpanMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_mapper_basic() {
        let mut mapper = SpanMapper::new();

        let span = TelemetryData::Span {
            span_id: "span-123".to_string(),
            trace_id: "trace-456".to_string(),
            parent_span_id: None,
            operation_name: "llm.generate".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            attributes: HashMap::new(),
            status: SpanStatus::Ok,
        };

        let entity = mapper.map_span(&span).unwrap();

        assert_eq!(entity.node_type, NodeType::Prompt);
        assert_eq!(entity.source_span_id, "span-123");
        assert_eq!(entity.source_trace_id, "trace-456");
        assert!(entity.session_id.is_some());
    }

    #[test]
    fn test_span_mapper_metadata_extraction() {
        let mut mapper = SpanMapper::new();

        let mut attributes = HashMap::new();
        attributes.insert("model".to_string(), "gpt-4".to_string());
        attributes.insert("temperature".to_string(), "0.7".to_string());
        attributes.insert("other".to_string(), "value".to_string());

        let span = TelemetryData::Span {
            span_id: "span-123".to_string(),
            trace_id: "trace-456".to_string(),
            parent_span_id: None,
            operation_name: "llm.completion".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            attributes,
            status: SpanStatus::Ok,
        };

        let entity = mapper.map_span(&span).unwrap();

        assert!(entity.metadata.contains_key("model"));
        assert!(entity.metadata.contains_key("temperature"));
        assert_eq!(entity.metadata.get("model").unwrap(), "gpt-4");
    }

    #[test]
    fn test_span_mapper_node_type_inference() {
        let mut mapper = SpanMapper::new();

        let test_cases = vec![
            ("llm.generate", NodeType::Prompt),
            ("llm.completion", NodeType::Response),
            ("tool.execute", NodeType::Tool),
            ("function.call", NodeType::Tool),
            ("agent.handoff", NodeType::Agent),
            ("unknown.operation", NodeType::Context),
        ];

        for (operation, expected_type) in test_cases {
            let span = TelemetryData::Span {
                span_id: "span-1".to_string(),
                trace_id: "trace-1".to_string(),
                parent_span_id: None,
                operation_name: operation.to_string(),
                start_time: Utc::now(),
                end_time: Utc::now(),
                attributes: HashMap::new(),
                status: SpanStatus::Ok,
            };

            let entity = mapper.map_span(&span).unwrap();
            assert_eq!(entity.node_type, expected_type, "Failed for operation: {}", operation);
        }
    }

    #[test]
    fn test_event_to_telemetry_node_created() {
        let mapper = SpanMapper::new();

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: Some(SessionId::new()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let telemetry = mapper.event_to_telemetry(&event).unwrap();

        if let TelemetryData::Span { operation_name, status, .. } = telemetry {
            assert!(operation_name.contains("Prompt"));
            assert_eq!(status, SpanStatus::Ok);
        } else {
            panic!("Expected span telemetry data");
        }
    }

    #[test]
    fn test_span_mapper_cache() {
        let mut mapper = SpanMapper::new();

        let span = TelemetryData::Span {
            span_id: "span-123".to_string(),
            trace_id: "trace-456".to_string(),
            parent_span_id: None,
            operation_name: "llm.generate".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            attributes: HashMap::new(),
            status: SpanStatus::Ok,
        };

        let entity1 = mapper.map_span(&span).unwrap();
        let entity2 = mapper.map_span(&span).unwrap();

        // Same span should map to same node ID
        assert_eq!(entity1.node_id, entity2.node_id);

        let node_id = mapper.get_node_id("span-123").unwrap();
        assert_eq!(node_id, entity1.node_id);

        mapper.clear_cache();
        assert!(mapper.get_node_id("span-123").is_none());
    }

    #[test]
    fn test_mapping_result() {
        let mut result = MappingResult::new();

        assert!(result.is_success());
        assert_eq!(result.entities.len(), 0);
        assert_eq!(result.edges.len(), 0);

        result.add_error("Test error".to_string());
        assert!(!result.is_success());
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_custom_mapping_config() {
        let mut patterns = HashMap::new();
        patterns.insert("custom.op".to_string(), NodeType::Tool);

        let config = MappingConfig {
            operation_patterns: patterns,
            metadata_keys: vec!["custom_key".to_string()],
            create_lineage_edges: false,
            trace_to_session: false,
        };

        let mut mapper = SpanMapper::with_config(config);

        let span = TelemetryData::Span {
            span_id: "span-1".to_string(),
            trace_id: "trace-1".to_string(),
            parent_span_id: None,
            operation_name: "custom.op".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            attributes: HashMap::new(),
            status: SpanStatus::Ok,
        };

        let entity = mapper.map_span(&span).unwrap();
        assert_eq!(entity.node_type, NodeType::Tool);
        assert!(entity.session_id.is_none()); // trace_to_session is false
    }
}
