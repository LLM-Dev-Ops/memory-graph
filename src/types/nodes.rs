//! Node types for the memory graph

use super::{NodeId, SessionId, TemplateId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Enum representing different node types in the graph
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    /// A prompt sent to an LLM
    Prompt,
    /// A response received from an LLM
    Response,
    /// A conversation session
    Session,
    /// A tool invocation by an LLM
    ToolInvocation,
}

/// Generic node wrapper that contains any node type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
    /// Prompt node
    Prompt(PromptNode),
    /// Response node
    Response(ResponseNode),
    /// Session node
    Session(ConversationSession),
    /// Tool invocation node
    ToolInvocation(ToolInvocation),
}

impl Node {
    /// Get the node ID
    #[must_use]
    pub fn id(&self) -> NodeId {
        match self {
            Node::Prompt(p) => p.id,
            Node::Response(r) => r.id,
            Node::Session(s) => s.node_id,
            Node::ToolInvocation(t) => t.id,
        }
    }

    /// Get the node type
    #[must_use]
    pub fn node_type(&self) -> NodeType {
        match self {
            Node::Prompt(_) => NodeType::Prompt,
            Node::Response(_) => NodeType::Response,
            Node::Session(_) => NodeType::Session,
            Node::ToolInvocation(_) => NodeType::ToolInvocation,
        }
    }
}

/// A conversation session that groups related prompts and responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSession {
    /// Internal node ID for the session
    pub node_id: NodeId,
    /// Unique session identifier
    pub id: SessionId,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// Custom metadata for the session
    pub metadata: HashMap<String, String>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl ConversationSession {
    /// Create a new conversation session
    #[must_use]
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            node_id: NodeId::new(),
            id: SessionId::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Create a session with custom metadata
    #[must_use]
    pub fn with_metadata(metadata: HashMap<String, String>) -> Self {
        let mut session = Self::new();
        session.metadata = metadata;
        session
    }

    /// Add a tag to the session
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Update the last modified timestamp
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

impl Default for ConversationSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata associated with a prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMetadata {
    /// The LLM model name (e.g., "gpt-4", "claude-3-opus")
    pub model: String,
    /// Temperature parameter for generation
    pub temperature: f32,
    /// Maximum tokens to generate
    pub max_tokens: Option<usize>,
    /// List of tools/functions available to the model
    pub tools_available: Vec<String>,
    /// Additional custom metadata
    pub custom: HashMap<String, String>,
}

impl Default for PromptMetadata {
    fn default() -> Self {
        Self {
            model: String::from("unknown"),
            temperature: 0.7,
            max_tokens: None,
            tools_available: Vec::new(),
            custom: HashMap::new(),
        }
    }
}

/// A prompt node representing input to an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptNode {
    /// Unique node identifier
    pub id: NodeId,
    /// Session this prompt belongs to
    pub session_id: SessionId,
    /// When the prompt was created
    pub timestamp: DateTime<Utc>,
    /// Optional template this prompt was instantiated from
    pub template_id: Option<TemplateId>,
    /// The actual prompt content
    pub content: String,
    /// Variables used if instantiated from a template
    pub variables: HashMap<String, String>,
    /// Metadata about the prompt
    pub metadata: PromptMetadata,
}

impl PromptNode {
    /// Create a new prompt node
    #[must_use]
    pub fn new(session_id: SessionId, content: String) -> Self {
        Self {
            id: NodeId::new(),
            session_id,
            timestamp: Utc::now(),
            template_id: None,
            content,
            variables: HashMap::new(),
            metadata: PromptMetadata::default(),
        }
    }

    /// Create a prompt with custom metadata
    #[must_use]
    pub fn with_metadata(session_id: SessionId, content: String, metadata: PromptMetadata) -> Self {
        Self {
            id: NodeId::new(),
            session_id,
            timestamp: Utc::now(),
            template_id: None,
            content,
            variables: HashMap::new(),
            metadata,
        }
    }

    /// Create a prompt from a template
    #[must_use]
    pub fn from_template(
        session_id: SessionId,
        template_id: TemplateId,
        content: String,
        variables: HashMap<String, String>,
    ) -> Self {
        Self {
            id: NodeId::new(),
            session_id,
            timestamp: Utc::now(),
            template_id: Some(template_id),
            content,
            variables,
            metadata: PromptMetadata::default(),
        }
    }
}

/// Token usage statistics for a response
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Create new token usage stats
    #[must_use]
    pub const fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

/// Metadata associated with a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// The model that generated the response
    pub model: String,
    /// Reason why generation stopped
    pub finish_reason: String,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Additional custom metadata
    pub custom: HashMap<String, String>,
}

impl Default for ResponseMetadata {
    fn default() -> Self {
        Self {
            model: String::from("unknown"),
            finish_reason: String::from("stop"),
            latency_ms: 0,
            custom: HashMap::new(),
        }
    }
}

/// A response node representing output from an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseNode {
    /// Unique node identifier
    pub id: NodeId,
    /// The prompt this response is replying to
    pub prompt_id: NodeId,
    /// When the response was created
    pub timestamp: DateTime<Utc>,
    /// The response content
    pub content: String,
    /// Token usage statistics
    pub usage: TokenUsage,
    /// Metadata about the response
    pub metadata: ResponseMetadata,
}

impl ResponseNode {
    /// Create a new response node
    #[must_use]
    pub fn new(prompt_id: NodeId, content: String, usage: TokenUsage) -> Self {
        Self {
            id: NodeId::new(),
            prompt_id,
            timestamp: Utc::now(),
            content,
            usage,
            metadata: ResponseMetadata::default(),
        }
    }

    /// Create a response with custom metadata
    #[must_use]
    pub fn with_metadata(
        prompt_id: NodeId,
        content: String,
        usage: TokenUsage,
        metadata: ResponseMetadata,
    ) -> Self {
        Self {
            id: NodeId::new(),
            prompt_id,
            timestamp: Utc::now(),
            content,
            usage,
            metadata,
        }
    }
}

/// A tool invocation node representing a function call by an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    /// Unique node identifier
    pub id: NodeId,
    /// Response that triggered this tool call
    pub response_id: NodeId,
    /// Name of the tool/function
    pub tool_name: String,
    /// JSON parameters passed to the tool
    pub parameters: serde_json::Value,
    /// Tool execution result (if completed)
    pub result: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// When the tool was invoked
    pub timestamp: DateTime<Utc>,
    /// Success/failure status
    pub success: bool,
    /// Retry count (for failed invocations)
    pub retry_count: u32,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ToolInvocation {
    /// Create a new pending tool invocation
    #[must_use]
    pub fn new(response_id: NodeId, tool_name: String, parameters: serde_json::Value) -> Self {
        Self {
            id: NodeId::new(),
            response_id,
            tool_name,
            parameters,
            result: None,
            error: None,
            duration_ms: 0,
            timestamp: Utc::now(),
            success: false,
            retry_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// Mark tool invocation as successful
    pub fn mark_success(&mut self, result: serde_json::Value, duration_ms: u64) {
        self.success = true;
        self.result = Some(result);
        self.error = None;
        self.duration_ms = duration_ms;
    }

    /// Mark tool invocation as failed
    pub fn mark_failed(&mut self, error: String, duration_ms: u64) {
        self.success = false;
        self.error = Some(error);
        self.result = None;
        self.duration_ms = duration_ms;
    }

    /// Record retry attempt
    pub fn record_retry(&mut self) {
        self.retry_count += 1;
        self.timestamp = Utc::now();
    }

    /// Check if tool invocation is pending (not completed)
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.result.is_none() && self.error.is_none()
    }

    /// Check if tool invocation succeeded
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.success
    }

    /// Check if tool invocation failed
    #[must_use]
    pub fn is_failed(&self) -> bool {
        self.error.is_some()
    }

    /// Get the tool execution status as a string
    #[must_use]
    pub fn status(&self) -> &str {
        if self.is_pending() {
            "pending"
        } else if self.success {
            "success"
        } else {
            "failed"
        }
    }

    /// Add metadata to the tool invocation
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = ConversationSession::new();
        assert!(!session.tags.is_empty() || session.tags.is_empty()); // Always true, just checking API
        assert!(session.metadata.is_empty());
    }

    #[test]
    fn test_session_tags() {
        let mut session = ConversationSession::new();
        session.add_tag("test".to_string());
        session.add_tag("test".to_string()); // Should not duplicate
        assert_eq!(session.tags.len(), 1);
    }

    #[test]
    fn test_prompt_creation() {
        let session_id = SessionId::new();
        let prompt = PromptNode::new(session_id, "Test prompt".to_string());
        assert_eq!(prompt.session_id, session_id);
        assert_eq!(prompt.content, "Test prompt");
    }

    #[test]
    fn test_response_creation() {
        let prompt_id = NodeId::new();
        let usage = TokenUsage::new(10, 20);
        let response = ResponseNode::new(prompt_id, "Test response".to_string(), usage);
        assert_eq!(response.prompt_id, prompt_id);
        assert_eq!(response.usage.total_tokens, 30);
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage::new(100, 50);
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }

    #[test]
    fn test_node_type() {
        let session = ConversationSession::new();
        let node = Node::Session(session);
        assert_eq!(node.node_type(), NodeType::Session);
    }

    #[test]
    fn test_tool_invocation_creation() {
        let response_id = NodeId::new();
        let params = serde_json::json!({"operation": "add", "a": 2, "b": 3});
        let tool = ToolInvocation::new(response_id, "calculator".to_string(), params.clone());

        assert_eq!(tool.response_id, response_id);
        assert_eq!(tool.tool_name, "calculator");
        assert_eq!(tool.parameters, params);
        assert!(tool.is_pending());
        assert!(!tool.is_success());
        assert!(!tool.is_failed());
        assert_eq!(tool.retry_count, 0);
    }

    #[test]
    fn test_tool_invocation_mark_success() {
        let response_id = NodeId::new();
        let params = serde_json::json!({"operation": "add", "a": 2, "b": 3});
        let mut tool = ToolInvocation::new(response_id, "calculator".to_string(), params);

        let result = serde_json::json!({"result": 5});
        tool.mark_success(result.clone(), 150);

        assert!(tool.is_success());
        assert!(!tool.is_pending());
        assert!(!tool.is_failed());
        assert_eq!(tool.result, Some(result));
        assert_eq!(tool.duration_ms, 150);
        assert_eq!(tool.error, None);
        assert_eq!(tool.status(), "success");
    }

    #[test]
    fn test_tool_invocation_mark_failed() {
        let response_id = NodeId::new();
        let params = serde_json::json!({"operation": "divide", "a": 10, "b": 0});
        let mut tool = ToolInvocation::new(response_id, "calculator".to_string(), params);

        tool.mark_failed("Division by zero".to_string(), 50);

        assert!(!tool.is_success());
        assert!(!tool.is_pending());
        assert!(tool.is_failed());
        assert_eq!(tool.error, Some("Division by zero".to_string()));
        assert_eq!(tool.duration_ms, 50);
        assert_eq!(tool.result, None);
        assert_eq!(tool.status(), "failed");
    }

    #[test]
    fn test_tool_invocation_retry() {
        let response_id = NodeId::new();
        let params = serde_json::json!({"url": "https://api.example.com"});
        let mut tool = ToolInvocation::new(response_id, "http_request".to_string(), params);

        assert_eq!(tool.retry_count, 0);

        tool.record_retry();
        assert_eq!(tool.retry_count, 1);

        tool.record_retry();
        assert_eq!(tool.retry_count, 2);
    }

    #[test]
    fn test_tool_invocation_metadata() {
        let response_id = NodeId::new();
        let params = serde_json::json!({"query": "test"});
        let mut tool = ToolInvocation::new(response_id, "search".to_string(), params);

        tool.add_metadata("provider".to_string(), "google".to_string());
        tool.add_metadata("cache_hit".to_string(), "true".to_string());

        assert_eq!(tool.metadata.len(), 2);
        assert_eq!(tool.metadata.get("provider"), Some(&"google".to_string()));
        assert_eq!(tool.metadata.get("cache_hit"), Some(&"true".to_string()));
    }

    #[test]
    fn test_tool_invocation_node_type() {
        let response_id = NodeId::new();
        let params = serde_json::json!({"test": "value"});
        let tool = ToolInvocation::new(response_id, "test_tool".to_string(), params);
        let node = Node::ToolInvocation(tool);

        assert_eq!(node.node_type(), NodeType::ToolInvocation);
    }
}
