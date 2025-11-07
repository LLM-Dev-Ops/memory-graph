//! Event types for Observatory integration
//!
//! This module defines all events that can be emitted by the memory graph
//! for real-time monitoring and analysis.

use crate::types::{
    AgentId, EdgeId, EdgeType, NodeId, NodeType, SessionId, TemplateId, TokenUsage,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Events emitted by the memory graph
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MemoryGraphEvent {
    /// Node created event
    NodeCreated {
        /// ID of the created node
        node_id: NodeId,
        /// Type of node created
        node_type: NodeType,
        /// Session ID (if applicable)
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<SessionId>,
        /// Event timestamp
        timestamp: DateTime<Utc>,
        /// Additional metadata
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        metadata: HashMap<String, String>,
    },

    /// Edge created event
    EdgeCreated {
        /// ID of the created edge
        edge_id: EdgeId,
        /// Type of edge created
        edge_type: EdgeType,
        /// Source node ID
        from: NodeId,
        /// Target node ID
        to: NodeId,
        /// Event timestamp
        timestamp: DateTime<Utc>,
    },

    /// Prompt submitted event
    PromptSubmitted {
        /// ID of the prompt
        prompt_id: NodeId,
        /// Session ID
        session_id: SessionId,
        /// Length of prompt content
        content_length: usize,
        /// Model used for the prompt
        model: String,
        /// Event timestamp
        timestamp: DateTime<Utc>,
    },

    /// Response generated event
    ResponseGenerated {
        /// ID of the response
        response_id: NodeId,
        /// ID of the prompt this responds to
        prompt_id: NodeId,
        /// Length of response content
        content_length: usize,
        /// Token usage statistics
        tokens_used: TokenUsage,
        /// Response generation latency in milliseconds
        latency_ms: u64,
        /// Event timestamp
        timestamp: DateTime<Utc>,
    },

    /// Tool invoked event
    ToolInvoked {
        /// ID of the tool invocation
        tool_id: NodeId,
        /// Name of the tool
        tool_name: String,
        /// Whether the tool invocation succeeded
        success: bool,
        /// Tool execution duration in milliseconds
        duration_ms: u64,
        /// Event timestamp
        timestamp: DateTime<Utc>,
    },

    /// Agent handoff event
    AgentHandoff {
        /// Agent transferring from
        from_agent: AgentId,
        /// Agent transferring to
        to_agent: AgentId,
        /// Session ID
        session_id: SessionId,
        /// Reason for handoff
        reason: String,
        /// Event timestamp
        timestamp: DateTime<Utc>,
    },

    /// Template instantiated event
    TemplateInstantiated {
        /// Template ID
        template_id: TemplateId,
        /// Prompt ID created from template
        prompt_id: NodeId,
        /// Template version used
        version: String,
        /// Variable bindings used
        variables: HashMap<String, String>,
        /// Event timestamp
        timestamp: DateTime<Utc>,
    },

    /// Query executed event
    QueryExecuted {
        /// Type of query executed
        query_type: String,
        /// Number of results returned
        results_count: usize,
        /// Query execution duration in milliseconds
        duration_ms: u64,
        /// Event timestamp
        timestamp: DateTime<Utc>,
    },
}

impl MemoryGraphEvent {
    /// Get a unique key for this event (for Kafka partitioning)
    pub fn key(&self) -> String {
        match self {
            Self::NodeCreated { node_id, .. } => format!("node:{}", node_id),
            Self::EdgeCreated { edge_id, .. } => format!("edge:{}", edge_id),
            Self::PromptSubmitted { session_id, .. } | Self::AgentHandoff { session_id, .. } => format!("session:{}", session_id),
            Self::ResponseGenerated { prompt_id, .. } => {
                format!("prompt:{}", prompt_id)
            }
            Self::ToolInvoked { tool_id, .. } => format!("tool:{}", tool_id),
            Self::TemplateInstantiated { template_id, .. } => format!("template:{}", template_id),
            Self::QueryExecuted { query_type, .. } => format!("query:{}", query_type),
        }
    }

    /// Get the event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::NodeCreated { .. } => "node_created",
            Self::EdgeCreated { .. } => "edge_created",
            Self::PromptSubmitted { .. } => "prompt_submitted",
            Self::ResponseGenerated { .. } => "response_generated",
            Self::ToolInvoked { .. } => "tool_invoked",
            Self::AgentHandoff { .. } => "agent_handoff",
            Self::TemplateInstantiated { .. } => "template_instantiated",
            Self::QueryExecuted { .. } => "query_executed",
        }
    }

    /// Get the timestamp of this event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::NodeCreated { timestamp, .. }
            | Self::EdgeCreated { timestamp, .. }
            | Self::PromptSubmitted { timestamp, .. }
            | Self::ResponseGenerated { timestamp, .. }
            | Self::ToolInvoked { timestamp, .. }
            | Self::AgentHandoff { timestamp, .. }
            | Self::TemplateInstantiated { timestamp, .. }
            | Self::QueryExecuted { timestamp, .. } => *timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeId;

    #[test]
    fn test_event_serialization() {
        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: Some(SessionId::new()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: MemoryGraphEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.event_type(), deserialized.event_type());
    }

    #[test]
    fn test_event_key_generation() {
        let node_id = NodeId::new();
        let event = MemoryGraphEvent::NodeCreated {
            node_id,
            node_type: NodeType::Prompt,
            session_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let key = event.key();
        assert!(key.starts_with("node:"));
        assert!(key.contains(&node_id.to_string()));
    }

    #[test]
    fn test_all_event_types() {
        let events = vec![
            MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Prompt,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            },
            MemoryGraphEvent::EdgeCreated {
                edge_id: EdgeId::new(),
                edge_type: EdgeType::Follows,
                from: NodeId::new(),
                to: NodeId::new(),
                timestamp: Utc::now(),
            },
            MemoryGraphEvent::PromptSubmitted {
                prompt_id: NodeId::new(),
                session_id: SessionId::new(),
                content_length: 100,
                model: "gpt-4".to_string(),
                timestamp: Utc::now(),
            },
            MemoryGraphEvent::ResponseGenerated {
                response_id: NodeId::new(),
                prompt_id: NodeId::new(),
                content_length: 200,
                tokens_used: TokenUsage::new(10, 20),
                latency_ms: 150,
                timestamp: Utc::now(),
            },
            MemoryGraphEvent::ToolInvoked {
                tool_id: NodeId::new(),
                tool_name: "calculator".to_string(),
                success: true,
                duration_ms: 50,
                timestamp: Utc::now(),
            },
            MemoryGraphEvent::AgentHandoff {
                from_agent: AgentId::new(),
                to_agent: AgentId::new(),
                session_id: SessionId::new(),
                reason: "specialized task".to_string(),
                timestamp: Utc::now(),
            },
            MemoryGraphEvent::TemplateInstantiated {
                template_id: TemplateId::new(),
                prompt_id: NodeId::new(),
                version: "1.0.0".to_string(),
                variables: HashMap::new(),
                timestamp: Utc::now(),
            },
            MemoryGraphEvent::QueryExecuted {
                query_type: "session_nodes".to_string(),
                results_count: 42,
                duration_ms: 25,
                timestamp: Utc::now(),
            },
        ];

        for event in events {
            assert!(!event.key().is_empty());
            assert!(!event.event_type().is_empty());
        }
    }

    #[test]
    fn test_tool_invoked_event() {
        let event = MemoryGraphEvent::ToolInvoked {
            tool_id: NodeId::new(),
            tool_name: "weather_api".to_string(),
            success: true,
            duration_ms: 250,
            timestamp: Utc::now(),
        };

        assert_eq!(event.event_type(), "tool_invoked");
        assert!(event.key().starts_with("tool:"));

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: MemoryGraphEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.event_type(), deserialized.event_type());
    }

    #[test]
    fn test_agent_handoff_event() {
        let from_agent = AgentId::new();
        let to_agent = AgentId::new();
        let session_id = SessionId::new();

        let event = MemoryGraphEvent::AgentHandoff {
            from_agent,
            to_agent,
            session_id,
            reason: "expertise required".to_string(),
            timestamp: Utc::now(),
        };

        assert_eq!(event.event_type(), "agent_handoff");
        assert!(event.key().contains(&session_id.to_string()));

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("expertise required"));
    }

    #[test]
    fn test_template_instantiated_event() {
        let template_id = TemplateId::new();
        let prompt_id = NodeId::new();
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "Alice".to_string());
        variables.insert("topic".to_string(), "AI".to_string());

        let event = MemoryGraphEvent::TemplateInstantiated {
            template_id,
            prompt_id,
            version: "2.1.0".to_string(),
            variables: variables.clone(),
            timestamp: Utc::now(),
        };

        assert_eq!(event.event_type(), "template_instantiated");
        assert!(event.key().contains(&template_id.to_string()));

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Alice"));
        assert!(json.contains("2.1.0"));
    }

    #[test]
    fn test_query_executed_event() {
        let event = MemoryGraphEvent::QueryExecuted {
            query_type: "filtered_search".to_string(),
            results_count: 128,
            duration_ms: 45,
            timestamp: Utc::now(),
        };

        assert_eq!(event.event_type(), "query_executed");
        assert!(event.key().contains("filtered_search"));

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: MemoryGraphEvent = serde_json::from_str(&json).unwrap();

        if let MemoryGraphEvent::QueryExecuted { results_count, .. } = deserialized {
            assert_eq!(results_count, 128);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_event_timestamp() {
        let timestamp = Utc::now();
        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Response,
            session_id: Some(SessionId::new()),
            timestamp,
            metadata: HashMap::new(),
        };

        assert_eq!(event.timestamp(), timestamp);
    }

    #[test]
    fn test_metadata_serialization() {
        let mut metadata = HashMap::new();
        metadata.insert("model".to_string(), "gpt-4".to_string());
        metadata.insert("temperature".to_string(), "0.7".to_string());

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: Some(SessionId::new()),
            timestamp: Utc::now(),
            metadata: metadata.clone(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("0.7"));

        let deserialized: MemoryGraphEvent = serde_json::from_str(&json).unwrap();
        if let MemoryGraphEvent::NodeCreated { metadata: meta, .. } = deserialized {
            assert_eq!(meta.get("model").unwrap(), "gpt-4");
            assert_eq!(meta.get("temperature").unwrap(), "0.7");
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_response_generated_with_token_usage() {
        let tokens = TokenUsage::new(150, 300);
        let event = MemoryGraphEvent::ResponseGenerated {
            response_id: NodeId::new(),
            prompt_id: NodeId::new(),
            content_length: 500,
            tokens_used: tokens,
            latency_ms: 1250,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: MemoryGraphEvent = serde_json::from_str(&json).unwrap();

        if let MemoryGraphEvent::ResponseGenerated {
            tokens_used,
            latency_ms,
            ..
        } = deserialized
        {
            assert_eq!(tokens_used.prompt_tokens, 150);
            assert_eq!(tokens_used.completion_tokens, 300);
            assert_eq!(latency_ms, 1250);
        } else {
            panic!("Wrong event type");
        }
    }
}
