//! Type conversion utilities between protobuf and internal types
//!
//! This module provides bidirectional conversion between protocol buffer
//! message types and internal Rust types used by the memory graph.

use crate::error::{Error, Result};
use crate::grpc::proto;
use crate::types::{
    ConversationSession, EdgeType, Node, NodeType, PromptMetadata, PromptNode, ResponseMetadata,
    ResponseNode, SessionId, TokenUsage, ToolInvocation, AgentNode, PromptTemplate, VariableSpec,
};
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use std::collections::HashMap;
use tonic::Status;

// ============================================================================
// Error Conversion
// ============================================================================

/// Convert internal Error to gRPC Status
pub fn error_to_status(err: Error) -> Status {
    match err {
        Error::NotFound(msg) => Status::not_found(msg),
        Error::InvalidInput(msg) => Status::invalid_argument(msg),
        Error::Timeout(msg) => Status::deadline_exceeded(msg),
        Error::Storage(msg) => Status::internal(format!("Storage error: {}", msg)),
        Error::Serialization(msg) => Status::internal(format!("Serialization error: {}", msg)),
        Error::SessionNotFound(msg) => Status::not_found(format!("Session not found: {}", msg)),
        Error::NodeNotFound(msg) => Status::not_found(format!("Node not found: {}", msg)),
        Error::IO(err) => Status::internal(format!("IO error: {}", err)),
        Error::Other(msg) => Status::internal(msg),
    }
}

/// Convert Result<T> to Result<T, Status>
pub fn result_to_status<T>(result: Result<T>) -> std::result::Result<T, Status> {
    result.map_err(error_to_status)
}

// ============================================================================
// Timestamp Conversion
// ============================================================================

/// Convert chrono DateTime to protobuf Timestamp
pub fn datetime_to_proto(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

/// Convert protobuf Timestamp to chrono DateTime
pub fn proto_to_datetime(ts: Timestamp) -> Result<DateTime<Utc>> {
    DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
        .ok_or_else(|| Error::InvalidInput("Invalid timestamp".to_string()))
}

/// Convert optional protobuf Timestamp to chrono DateTime
pub fn optional_proto_to_datetime(ts: Option<Timestamp>) -> Result<DateTime<Utc>> {
    match ts {
        Some(timestamp) => proto_to_datetime(timestamp),
        None => Err(Error::InvalidInput("Missing timestamp".to_string())),
    }
}

// ============================================================================
// Session Conversion
// ============================================================================

/// Convert internal ConversationSession to protobuf Session
pub fn session_to_proto(session: ConversationSession) -> proto::Session {
    proto::Session {
        id: session.id.to_string(),
        created_at: Some(datetime_to_proto(session.created_at)),
        updated_at: Some(datetime_to_proto(session.updated_at)),
        metadata: session.metadata,
        is_active: session.is_active,
    }
}

// ============================================================================
// Node Type Conversion
// ============================================================================

/// Convert protobuf NodeType to internal NodeType
pub fn proto_to_node_type(node_type: i32) -> Result<NodeType> {
    match proto::NodeType::try_from(node_type) {
        Ok(proto::NodeType::NodeTypeSession) => Ok(NodeType::Session),
        Ok(proto::NodeType::NodeTypePrompt) => Ok(NodeType::Prompt),
        Ok(proto::NodeType::NodeTypeResponse) => Ok(NodeType::Response),
        Ok(proto::NodeType::NodeTypeToolInvocation) => Ok(NodeType::ToolInvocation),
        Ok(proto::NodeType::NodeTypeAgent) => Ok(NodeType::Agent),
        Ok(proto::NodeType::NodeTypeTemplate) => Ok(NodeType::Template),
        _ => Err(Error::InvalidInput(format!("Invalid node type: {}", node_type))),
    }
}

/// Convert internal NodeType to protobuf NodeType
pub fn node_type_to_proto(node_type: NodeType) -> i32 {
    match node_type {
        NodeType::Session => proto::NodeType::NodeTypeSession as i32,
        NodeType::Prompt => proto::NodeType::NodeTypePrompt as i32,
        NodeType::Response => proto::NodeType::NodeTypeResponse as i32,
        NodeType::ToolInvocation => proto::NodeType::NodeTypeToolInvocation as i32,
        NodeType::Agent => proto::NodeType::NodeTypeAgent as i32,
        NodeType::Template => proto::NodeType::NodeTypeTemplate as i32,
    }
}

// ============================================================================
// Edge Type Conversion
// ============================================================================

/// Convert protobuf EdgeType to internal EdgeType
pub fn proto_to_edge_type(edge_type: i32) -> Result<EdgeType> {
    match proto::EdgeType::try_from(edge_type) {
        Ok(proto::EdgeType::EdgeTypeBelongsTo) => Ok(EdgeType::PartOf),
        Ok(proto::EdgeType::EdgeTypeRespondsTo) => Ok(EdgeType::RespondsTo),
        Ok(proto::EdgeType::EdgeTypeFollows) => Ok(EdgeType::Follows),
        Ok(proto::EdgeType::EdgeTypeInvokes) => Ok(EdgeType::Invokes),
        Ok(proto::EdgeType::EdgeTypeHandledBy) => Ok(EdgeType::HandledBy),
        Ok(proto::EdgeType::EdgeTypeInstantiates) => Ok(EdgeType::Instantiates),
        Ok(proto::EdgeType::EdgeTypeInherits) => Ok(EdgeType::Inherits),
        Ok(proto::EdgeType::EdgeTypeTransfersTo) => Ok(EdgeType::TransfersTo),
        Ok(proto::EdgeType::EdgeTypeReferences) => Ok(EdgeType::References),
        _ => Err(Error::InvalidInput(format!("Invalid edge type: {}", edge_type))),
    }
}

/// Convert internal EdgeType to protobuf EdgeType
pub fn edge_type_to_proto(edge_type: EdgeType) -> i32 {
    match edge_type {
        EdgeType::PartOf => proto::EdgeType::EdgeTypeBelongsTo as i32,
        EdgeType::RespondsTo => proto::EdgeType::EdgeTypeRespondsTo as i32,
        EdgeType::Follows => proto::EdgeType::EdgeTypeFollows as i32,
        EdgeType::Invokes => proto::EdgeType::EdgeTypeInvokes as i32,
        EdgeType::HandledBy => proto::EdgeType::EdgeTypeHandledBy as i32,
        EdgeType::Instantiates => proto::EdgeType::EdgeTypeInstantiates as i32,
        EdgeType::Inherits => proto::EdgeType::EdgeTypeInherits as i32,
        EdgeType::TransfersTo => proto::EdgeType::EdgeTypeTransfersTo as i32,
        EdgeType::References => proto::EdgeType::EdgeTypeReferences as i32,
    }
}

// ============================================================================
// Metadata Conversion
// ============================================================================

/// Convert protobuf PromptMetadata to internal PromptMetadata
pub fn proto_to_prompt_metadata(metadata: proto::PromptMetadata) -> PromptMetadata {
    PromptMetadata {
        model: metadata.model,
        temperature: metadata.temperature,
        max_tokens: metadata.max_tokens.map(|t| t as usize),
        tools_available: metadata.tools_available,
        custom: metadata.custom,
    }
}

/// Convert internal PromptMetadata to protobuf PromptMetadata
pub fn prompt_metadata_to_proto(metadata: PromptMetadata) -> proto::PromptMetadata {
    proto::PromptMetadata {
        model: metadata.model,
        temperature: metadata.temperature,
        max_tokens: metadata.max_tokens.map(|t| t as i32),
        tools_available: metadata.tools_available,
        custom: metadata.custom,
    }
}

/// Convert protobuf ResponseMetadata to internal ResponseMetadata
pub fn proto_to_response_metadata(metadata: proto::ResponseMetadata) -> ResponseMetadata {
    ResponseMetadata {
        model: metadata.model,
        finish_reason: metadata.finish_reason,
        latency_ms: metadata.latency_ms as u64,
        custom: metadata.custom,
    }
}

/// Convert internal ResponseMetadata to protobuf ResponseMetadata
pub fn response_metadata_to_proto(metadata: ResponseMetadata) -> proto::ResponseMetadata {
    proto::ResponseMetadata {
        model: metadata.model,
        finish_reason: metadata.finish_reason,
        latency_ms: metadata.latency_ms as i64,
        custom: metadata.custom,
    }
}

/// Convert protobuf TokenUsage to internal TokenUsage
pub fn proto_to_token_usage(usage: proto::TokenUsage) -> TokenUsage {
    TokenUsage {
        prompt_tokens: usage.prompt_tokens as usize,
        completion_tokens: usage.completion_tokens as usize,
        total_tokens: usage.total_tokens as usize,
    }
}

/// Convert internal TokenUsage to protobuf TokenUsage
pub fn token_usage_to_proto(usage: TokenUsage) -> proto::TokenUsage {
    proto::TokenUsage {
        prompt_tokens: usage.prompt_tokens as i64,
        completion_tokens: usage.completion_tokens as i64,
        total_tokens: usage.total_tokens as i64,
    }
}

// ============================================================================
// Node Conversion
// ============================================================================

/// Convert internal PromptNode to protobuf PromptNode
pub fn prompt_node_to_proto(prompt: PromptNode) -> proto::PromptNode {
    proto::PromptNode {
        id: prompt.id.to_string(),
        session_id: prompt.session_id.to_string(),
        content: prompt.content,
        timestamp: Some(datetime_to_proto(prompt.timestamp)),
        metadata: Some(prompt_metadata_to_proto(prompt.metadata)),
    }
}

/// Convert internal ResponseNode to protobuf ResponseNode
pub fn response_node_to_proto(response: ResponseNode) -> proto::ResponseNode {
    proto::ResponseNode {
        id: response.id.to_string(),
        prompt_id: response.prompt_id.to_string(),
        content: response.content,
        timestamp: Some(datetime_to_proto(response.timestamp)),
        token_usage: Some(token_usage_to_proto(response.usage)),
        metadata: Some(response_metadata_to_proto(response.metadata)),
    }
}

/// Convert internal ToolInvocation to protobuf ToolInvocationNode
pub fn tool_invocation_to_proto(tool: ToolInvocation) -> proto::ToolInvocationNode {
    proto::ToolInvocationNode {
        id: tool.id.to_string(),
        response_id: tool.response_id.to_string(),
        tool_name: tool.tool_name,
        parameters: tool.parameters.to_string(),
        status: tool.status,
        result: tool.result.map(|r| r.to_string()),
        error: tool.error,
        duration_ms: tool.duration_ms.map(|d| d as i64).unwrap_or(0),
        retry_count: tool.retry_count.unwrap_or(0) as i32,
        timestamp: Some(datetime_to_proto(tool.timestamp)),
        metadata: tool.metadata,
    }
}

/// Convert internal AgentNode to protobuf AgentNode
pub fn agent_node_to_proto(agent: AgentNode) -> proto::AgentNode {
    proto::AgentNode {
        id: agent.id.to_string(),
        name: agent.name,
        role: agent.role,
        capabilities: agent.capabilities,
        status: agent.status.to_string(),
        created_at: Some(datetime_to_proto(agent.created_at)),
        metadata: agent.metadata,
    }
}

/// Convert internal PromptTemplate to protobuf TemplateNode
pub fn template_to_proto(template: PromptTemplate) -> proto::TemplateNode {
    proto::TemplateNode {
        id: template.id.to_string(),
        name: template.name,
        template_text: template.template_text,
        variables: template
            .variables
            .into_iter()
            .map(variable_spec_to_proto)
            .collect(),
        version: template.version.to_string(),
        usage_count: template.usage_count as i64,
        created_at: Some(datetime_to_proto(template.created_at)),
        metadata: template.metadata,
    }
}

/// Convert internal VariableSpec to protobuf VariableSpec
pub fn variable_spec_to_proto(spec: VariableSpec) -> proto::VariableSpec {
    proto::VariableSpec {
        name: spec.name,
        type_hint: spec.type_hint,
        required: spec.required,
        default_value: spec.default_value,
        validation_pattern: spec.validation_pattern,
        description: spec.description,
    }
}

/// Convert internal Node to protobuf Node
pub fn node_to_proto(node: Node) -> proto::Node {
    let id = node.id().to_string();
    let created_at = Some(datetime_to_proto(Utc::now())); // TODO: Get actual created_at from node

    match node {
        Node::Prompt(prompt) => {
            let node_type = node_type_to_proto(NodeType::Prompt);
            proto::Node {
                id,
                r#type: node_type,
                created_at,
                node_data: Some(proto::node::NodeData::Prompt(prompt_node_to_proto(prompt))),
            }
        }
        Node::Response(response) => {
            let node_type = node_type_to_proto(NodeType::Response);
            proto::Node {
                id,
                r#type: node_type,
                created_at,
                node_data: Some(proto::node::NodeData::Response(response_node_to_proto(response))),
            }
        }
        Node::ToolInvocation(tool) => {
            let node_type = node_type_to_proto(NodeType::ToolInvocation);
            proto::Node {
                id,
                r#type: node_type,
                created_at,
                node_data: Some(proto::node::NodeData::ToolInvocation(tool_invocation_to_proto(tool))),
            }
        }
        Node::Agent(agent) => {
            let node_type = node_type_to_proto(NodeType::Agent);
            proto::Node {
                id,
                r#type: node_type,
                created_at,
                node_data: Some(proto::node::NodeData::Agent(agent_node_to_proto(agent))),
            }
        }
        Node::Template(template) => {
            let node_type = node_type_to_proto(NodeType::Template);
            proto::Node {
                id,
                r#type: node_type,
                created_at,
                node_data: Some(proto::node::NodeData::Template(template_to_proto(template))),
            }
        }
        Node::Session(session) => {
            let node_type = node_type_to_proto(NodeType::Session);
            proto::Node {
                id,
                r#type: node_type,
                created_at,
                node_data: None, // Session doesn't use node_data field
            }
        }
    }
}

// ============================================================================
// Edge Conversion
// ============================================================================

/// Convert internal Edge to protobuf Edge
pub fn edge_to_proto(edge: crate::types::Edge) -> proto::Edge {
    proto::Edge {
        id: edge.id.to_string(),
        from_node_id: edge.from.to_string(),
        to_node_id: edge.to.to_string(),
        r#type: edge_type_to_proto(edge.edge_type),
        created_at: Some(datetime_to_proto(edge.created_at)),
        properties: edge.properties.unwrap_or_default(),
    }
}

// ============================================================================
// SessionId Parsing
// ============================================================================

/// Parse SessionId from string
pub fn parse_session_id(id: &str) -> Result<SessionId> {
    SessionId::parse_str(id)
        .map_err(|_| Error::InvalidInput(format!("Invalid session ID: {}", id)))
}

/// Parse NodeId from string
pub fn parse_node_id(id: &str) -> Result<crate::types::NodeId> {
    crate::types::NodeId::parse_str(id)
        .map_err(|_| Error::InvalidInput(format!("Invalid node ID: {}", id)))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_conversion() {
        let now = Utc::now();
        let proto_ts = datetime_to_proto(now);
        let converted = proto_to_datetime(proto_ts).unwrap();

        // Allow for millisecond precision differences
        assert_eq!(now.timestamp(), converted.timestamp());
    }

    #[test]
    fn test_node_type_conversion() {
        assert_eq!(node_type_to_proto(NodeType::Prompt), proto::NodeType::NodeTypePrompt as i32);
        assert_eq!(
            proto_to_node_type(proto::NodeType::NodeTypePrompt as i32).unwrap(),
            NodeType::Prompt
        );
    }

    #[test]
    fn test_edge_type_conversion() {
        assert_eq!(
            edge_type_to_proto(EdgeType::RespondsTo),
            proto::EdgeType::EdgeTypeRespondsTo as i32
        );
        assert_eq!(
            proto_to_edge_type(proto::EdgeType::EdgeTypeRespondsTo as i32).unwrap(),
            EdgeType::RespondsTo
        );
    }

    #[test]
    fn test_token_usage_conversion() {
        let usage = TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 50,
            total_tokens: 60,
        };

        let proto_usage = token_usage_to_proto(usage);
        let converted = proto_to_token_usage(proto_usage);

        assert_eq!(converted.prompt_tokens, 10);
        assert_eq!(converted.completion_tokens, 50);
        assert_eq!(converted.total_tokens, 60);
    }
}
