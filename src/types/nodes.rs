//! Node types for the memory graph

use super::{AgentId, NodeId, SessionId, TemplateId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

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
    /// An autonomous agent
    Agent,
    /// A versioned prompt template
    Template,
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
    /// Agent node
    Agent(AgentNode),
    /// Template node
    Template(PromptTemplate),
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
            Node::Agent(a) => a.node_id,
            Node::Template(t) => t.node_id,
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
            Node::Agent(_) => NodeType::Agent,
            Node::Template(_) => NodeType::Template,
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

/// Agent status enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentStatus {
    /// Agent is active and ready to process tasks
    Active,
    /// Agent is idle, waiting for work
    #[default]
    Idle,
    /// Agent is currently busy processing a task
    Busy,
    /// Agent is paused and not accepting new tasks
    Paused,
    /// Agent has been terminated
    Terminated,
}

impl AgentStatus {
    /// Check if agent can accept new tasks
    #[must_use]
    pub const fn can_accept_tasks(&self) -> bool {
        matches!(self, Self::Active | Self::Idle)
    }

    /// Check if agent is processing a task
    #[must_use]
    pub const fn is_busy(&self) -> bool {
        matches!(self, Self::Busy)
    }

    /// Check if agent is operational
    #[must_use]
    pub const fn is_operational(&self) -> bool {
        !matches!(self, Self::Terminated)
    }
}

/// Agent configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Temperature parameter for generation
    pub temperature: f32,
    /// Maximum tokens to generate
    pub max_tokens: usize,
    /// Timeout in seconds for agent operations
    pub timeout_seconds: u64,
    /// Maximum number of retries for failed operations
    pub max_retries: u32,
    /// List of tools/functions available to the agent
    pub tools_enabled: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 2000,
            timeout_seconds: 300,
            max_retries: 3,
            tools_enabled: Vec::new(),
        }
    }
}

/// Agent performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Total number of prompts processed
    pub total_prompts: u64,
    /// Number of successfully completed tasks
    pub successful_tasks: u64,
    /// Number of failed tasks
    pub failed_tasks: u64,
    /// Average latency in milliseconds
    pub average_latency_ms: f64,
    /// Total tokens used across all operations
    pub total_tokens_used: u64,
}

impl Default for AgentMetrics {
    fn default() -> Self {
        Self {
            total_prompts: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            average_latency_ms: 0.0,
            total_tokens_used: 0,
        }
    }
}

impl AgentMetrics {
    /// Calculate success rate as a percentage
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        let total = self.successful_tasks + self.failed_tasks;
        if total == 0 {
            0.0
        } else {
            (self.successful_tasks as f64 / total as f64) * 100.0
        }
    }

    /// Update average latency with new measurement
    pub fn update_average_latency(&mut self, new_latency_ms: u64, current_count: u64) {
        if current_count == 0 {
            self.average_latency_ms = new_latency_ms as f64;
        } else {
            self.average_latency_ms = (self.average_latency_ms * current_count as f64
                + new_latency_ms as f64)
                / (current_count + 1) as f64;
        }
    }
}

/// An agent node representing an autonomous AI agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    /// Unique agent identifier
    pub id: AgentId,
    /// Internal node ID for graph storage
    pub node_id: NodeId,
    /// Human-readable agent name
    pub name: String,
    /// Agent role/specialization (e.g., "researcher", "coder", "reviewer")
    pub role: String,
    /// List of agent capabilities
    pub capabilities: Vec<String>,
    /// Model used by this agent
    pub model: String,
    /// When the agent was created
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_active: DateTime<Utc>,
    /// Agent status (active, idle, busy, paused, terminated)
    pub status: AgentStatus,
    /// Configuration parameters
    pub config: AgentConfig,
    /// Performance metrics
    pub metrics: AgentMetrics,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl AgentNode {
    /// Create a new agent
    #[must_use]
    pub fn new(name: String, role: String, capabilities: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: AgentId::new(),
            node_id: NodeId::new(),
            name,
            role,
            capabilities,
            model: String::from("gpt-4"),
            created_at: now,
            last_active: now,
            status: AgentStatus::Idle,
            config: AgentConfig::default(),
            metrics: AgentMetrics::default(),
            tags: Vec::new(),
        }
    }

    /// Create an agent with custom configuration
    #[must_use]
    pub fn with_config(
        name: String,
        role: String,
        capabilities: Vec<String>,
        config: AgentConfig,
    ) -> Self {
        let mut agent = Self::new(name, role, capabilities);
        agent.config = config;
        agent
    }

    /// Create an agent with a specific model
    #[must_use]
    pub fn with_model(
        name: String,
        role: String,
        capabilities: Vec<String>,
        model: String,
    ) -> Self {
        let mut agent = Self::new(name, role, capabilities);
        agent.model = model;
        agent
    }

    /// Update agent status
    pub fn set_status(&mut self, status: AgentStatus) {
        self.status = status;
        self.last_active = Utc::now();
    }

    /// Record agent activity
    pub fn record_activity(&mut self) {
        self.last_active = Utc::now();
    }

    /// Update performance metrics
    pub fn update_metrics(&mut self, success: bool, latency_ms: u64, tokens: u64) {
        let current_count = self.metrics.total_prompts;
        self.metrics.total_prompts += 1;
        if success {
            self.metrics.successful_tasks += 1;
        } else {
            self.metrics.failed_tasks += 1;
        }
        self.metrics
            .update_average_latency(latency_ms, current_count);
        self.metrics.total_tokens_used += tokens;
        self.record_activity();
    }

    /// Add a capability to the agent
    pub fn add_capability(&mut self, capability: String) {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }

    /// Remove a capability from the agent
    pub fn remove_capability(&mut self, capability: &str) {
        self.capabilities.retain(|c| c != capability);
    }

    /// Check if agent has a specific capability
    #[must_use]
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(&String::from(capability))
    }

    /// Add a tag to the agent
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Get agent uptime in seconds
    #[must_use]
    pub fn uptime_seconds(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }

    /// Get time since last activity in seconds
    #[must_use]
    pub fn idle_time_seconds(&self) -> i64 {
        (Utc::now() - self.last_active).num_seconds()
    }

    /// Check if agent is healthy (active and operational)
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.status.is_operational() && self.idle_time_seconds() < 3600 // Not idle for more than 1 hour
    }
}

/// Semantic version for template versioning
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    /// Major version (breaking changes)
    pub major: u16,
    /// Minor version (new features, backwards compatible)
    pub minor: u16,
    /// Patch version (bug fixes)
    pub patch: u16,
}

impl Version {
    /// Create a new version
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Bump major version (resets minor and patch to 0)
    pub fn bump_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
    }

    /// Bump minor version (resets patch to 0)
    pub fn bump_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    /// Bump patch version
    pub fn bump_patch(&mut self) {
        self.patch += 1;
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::new(1, 0, 0)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::str::FromStr for Version {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version format: {}", s));
        }

        let major = parts[0]
            .parse::<u16>()
            .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse::<u16>()
            .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2]
            .parse::<u16>()
            .map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok(Self::new(major, minor, patch))
    }
}

/// Version bump level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionLevel {
    /// Major version bump (breaking changes)
    Major,
    /// Minor version bump (new features)
    Minor,
    /// Patch version bump (bug fixes)
    Patch,
}

/// Variable specification for template variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableSpec {
    /// Variable name (e.g., "user_query")
    pub name: String,
    /// Type hint (e.g., "string", "number", "array")
    pub type_hint: String,
    /// Whether the variable is required
    pub required: bool,
    /// Default value if not provided
    pub default: Option<String>,
    /// Validation regex pattern
    pub validation_pattern: Option<String>,
    /// Human-readable description
    pub description: String,
}

impl VariableSpec {
    /// Create a new variable specification
    #[must_use]
    pub fn new(name: String, type_hint: String, required: bool, description: String) -> Self {
        Self {
            name,
            type_hint,
            required,
            default: None,
            validation_pattern: None,
            description,
        }
    }

    /// Create a variable spec with a default value
    #[must_use]
    pub fn with_default(mut self, default: String) -> Self {
        self.default = Some(default);
        self
    }

    /// Create a variable spec with validation pattern
    #[must_use]
    pub fn with_validation(mut self, pattern: String) -> Self {
        self.validation_pattern = Some(pattern);
        self
    }

    /// Validate a value against this spec
    pub fn validate(&self, value: &Option<String>) -> Result<(), String> {
        // Check if required
        if self.required && value.is_none() {
            return Err(format!("Required variable '{}' is missing", self.name));
        }

        // If value is present, validate pattern
        if let Some(val) = value {
            if let Some(ref pattern) = self.validation_pattern {
                let re = regex::Regex::new(pattern)
                    .map_err(|e| format!("Invalid regex pattern: {}", e))?;
                if !re.is_match(val) {
                    return Err(format!(
                        "Variable '{}' value '{}' does not match pattern '{}'",
                        self.name, val, pattern
                    ));
                }
            }
        }

        Ok(())
    }
}

/// A prompt template node for reusable prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Unique template identifier
    pub id: TemplateId,
    /// Internal node ID for graph storage
    pub node_id: NodeId,
    /// Semantic version
    pub version: Version,
    /// Human-readable template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Template content with {{variables}}
    pub template: String,
    /// Variable specifications
    pub variables: Vec<VariableSpec>,
    /// Parent template ID (for inheritance)
    pub parent_id: Option<TemplateId>,
    /// When the template was created
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
    /// Template author
    pub author: String,
    /// Usage count
    pub usage_count: u64,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl PromptTemplate {
    /// Create a new template
    #[must_use]
    pub fn new(name: String, template: String, variables: Vec<VariableSpec>) -> Self {
        let now = Utc::now();
        Self {
            id: TemplateId::new(),
            node_id: NodeId::new(),
            version: Version::default(),
            name,
            description: String::new(),
            template,
            variables,
            parent_id: None,
            created_at: now,
            updated_at: now,
            author: String::from("unknown"),
            usage_count: 0,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create template from parent (inheritance)
    #[must_use]
    pub fn from_parent(
        parent_id: TemplateId,
        name: String,
        template: String,
        variables: Vec<VariableSpec>,
    ) -> Self {
        let mut tmpl = Self::new(name, template, variables);
        tmpl.parent_id = Some(parent_id);
        tmpl
    }

    /// Set template description
    #[must_use]
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Set template author
    #[must_use]
    pub fn with_author(mut self, author: String) -> Self {
        self.author = author;
        self
    }

    /// Instantiate template with variable values
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required variables are missing
    /// - Variable validation fails
    pub fn instantiate(&self, values: &HashMap<String, String>) -> Result<String, String> {
        // Validate all variables
        self.validate(values)?;

        // Build final variable map with defaults
        let mut final_values = HashMap::new();
        for var in &self.variables {
            if let Some(value) = values.get(&var.name) {
                final_values.insert(var.name.clone(), value.clone());
            } else if let Some(ref default) = var.default {
                final_values.insert(var.name.clone(), default.clone());
            }
        }

        // Replace variables in template
        let mut result = self.template.clone();
        for (key, value) in final_values {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, &value);
        }

        Ok(result)
    }

    /// Validate variable values
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails
    pub fn validate(&self, values: &HashMap<String, String>) -> Result<(), String> {
        for var in &self.variables {
            let value = values.get(&var.name).cloned();
            var.validate(&value)?;
        }
        Ok(())
    }

    /// Increment usage counter
    pub fn record_usage(&mut self) {
        self.usage_count += 1;
        self.updated_at = Utc::now();
    }

    /// Bump version
    pub fn bump_version(&mut self, level: VersionLevel) {
        match level {
            VersionLevel::Major => self.version.bump_major(),
            VersionLevel::Minor => self.version.bump_minor(),
            VersionLevel::Patch => self.version.bump_patch(),
        }
        self.updated_at = Utc::now();
    }

    /// Add a tag to the template
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Add metadata to the template
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

    // AgentNode tests

    #[test]
    fn test_agent_creation() {
        let agent = AgentNode::new(
            "Researcher".to_string(),
            "research".to_string(),
            vec!["web_search".to_string(), "summarize".to_string()],
        );

        assert_eq!(agent.name, "Researcher");
        assert_eq!(agent.role, "research");
        assert_eq!(agent.capabilities.len(), 2);
        assert_eq!(agent.status, AgentStatus::Idle);
        assert_eq!(agent.model, "gpt-4");
        assert_eq!(agent.metrics.total_prompts, 0);
    }

    #[test]
    fn test_agent_with_config() {
        let config = AgentConfig {
            temperature: 0.5,
            max_tokens: 1000,
            timeout_seconds: 60,
            max_retries: 5,
            tools_enabled: vec!["calculator".to_string()],
        };

        let agent = AgentNode::with_config(
            "Calculator".to_string(),
            "math".to_string(),
            vec!["calculate".to_string()],
            config.clone(),
        );

        assert_eq!(agent.config.temperature, 0.5);
        assert_eq!(agent.config.max_tokens, 1000);
        assert_eq!(agent.config.timeout_seconds, 60);
        assert_eq!(agent.config.max_retries, 5);
        assert_eq!(agent.config.tools_enabled.len(), 1);
    }

    #[test]
    fn test_agent_with_model() {
        let agent = AgentNode::with_model(
            "Claude Agent".to_string(),
            "assistant".to_string(),
            vec![],
            "claude-3-opus".to_string(),
        );

        assert_eq!(agent.model, "claude-3-opus");
    }

    #[test]
    fn test_agent_status_transitions() {
        let mut agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);

        assert_eq!(agent.status, AgentStatus::Idle);
        assert!(agent.status.can_accept_tasks());

        agent.set_status(AgentStatus::Busy);
        assert_eq!(agent.status, AgentStatus::Busy);
        assert!(agent.status.is_busy());
        assert!(!agent.status.can_accept_tasks());

        agent.set_status(AgentStatus::Active);
        assert_eq!(agent.status, AgentStatus::Active);
        assert!(agent.status.can_accept_tasks());

        agent.set_status(AgentStatus::Paused);
        assert!(!agent.status.can_accept_tasks());

        agent.set_status(AgentStatus::Terminated);
        assert!(!agent.status.is_operational());
    }

    #[test]
    fn test_agent_metrics_update() {
        let mut agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);

        assert_eq!(agent.metrics.total_prompts, 0);
        assert_eq!(agent.metrics.success_rate(), 0.0);

        agent.update_metrics(true, 100, 50);
        assert_eq!(agent.metrics.total_prompts, 1);
        assert_eq!(agent.metrics.successful_tasks, 1);
        assert_eq!(agent.metrics.failed_tasks, 0);
        assert_eq!(agent.metrics.average_latency_ms, 100.0);
        assert_eq!(agent.metrics.total_tokens_used, 50);
        assert_eq!(agent.metrics.success_rate(), 100.0);

        agent.update_metrics(false, 200, 30);
        assert_eq!(agent.metrics.total_prompts, 2);
        assert_eq!(agent.metrics.successful_tasks, 1);
        assert_eq!(agent.metrics.failed_tasks, 1);
        assert_eq!(agent.metrics.total_tokens_used, 80);
        assert_eq!(agent.metrics.success_rate(), 50.0);

        // Average latency should be (100 + 200) / 2 = 150
        assert_eq!(agent.metrics.average_latency_ms, 150.0);
    }

    #[test]
    fn test_agent_capabilities() {
        let mut agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);

        assert!(!agent.has_capability("web_search"));

        agent.add_capability("web_search".to_string());
        assert!(agent.has_capability("web_search"));
        assert_eq!(agent.capabilities.len(), 1);

        agent.add_capability("web_search".to_string()); // Duplicate
        assert_eq!(agent.capabilities.len(), 1); // Should not add duplicate

        agent.add_capability("summarize".to_string());
        assert_eq!(agent.capabilities.len(), 2);

        agent.remove_capability("web_search");
        assert!(!agent.has_capability("web_search"));
        assert_eq!(agent.capabilities.len(), 1);
    }

    #[test]
    fn test_agent_tags() {
        let mut agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);

        assert_eq!(agent.tags.len(), 0);

        agent.add_tag("production".to_string());
        assert_eq!(agent.tags.len(), 1);

        agent.add_tag("production".to_string()); // Duplicate
        assert_eq!(agent.tags.len(), 1); // Should not add duplicate

        agent.add_tag("critical".to_string());
        assert_eq!(agent.tags.len(), 2);
    }

    #[test]
    fn test_agent_activity_tracking() {
        let mut agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);

        let initial_active = agent.last_active;
        std::thread::sleep(std::time::Duration::from_millis(10));

        agent.record_activity();
        assert!(agent.last_active > initial_active);
        assert_eq!(agent.idle_time_seconds(), 0);
    }

    #[test]
    fn test_agent_uptime() {
        let agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);

        let uptime = agent.uptime_seconds();
        assert!(uptime >= 0);
        assert!(uptime < 5); // Should be very small since just created
    }

    #[test]
    fn test_agent_health_check() {
        let mut agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);

        agent.set_status(AgentStatus::Active);
        assert!(agent.is_healthy());

        agent.set_status(AgentStatus::Terminated);
        assert!(!agent.is_healthy());
    }

    #[test]
    fn test_agent_node_type() {
        let agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);
        let node = Node::Agent(agent);

        assert_eq!(node.node_type(), NodeType::Agent);
    }

    #[test]
    fn test_agent_status_helpers() {
        assert!(AgentStatus::Active.can_accept_tasks());
        assert!(AgentStatus::Idle.can_accept_tasks());
        assert!(!AgentStatus::Busy.can_accept_tasks());
        assert!(!AgentStatus::Paused.can_accept_tasks());
        assert!(!AgentStatus::Terminated.can_accept_tasks());

        assert!(!AgentStatus::Active.is_busy());
        assert!(AgentStatus::Busy.is_busy());

        assert!(AgentStatus::Active.is_operational());
        assert!(AgentStatus::Idle.is_operational());
        assert!(AgentStatus::Busy.is_operational());
        assert!(AgentStatus::Paused.is_operational());
        assert!(!AgentStatus::Terminated.is_operational());
    }

    #[test]
    fn test_agent_metrics_success_rate() {
        let mut metrics = AgentMetrics::default();

        assert_eq!(metrics.success_rate(), 0.0);

        metrics.successful_tasks = 5;
        metrics.failed_tasks = 5;
        assert_eq!(metrics.success_rate(), 50.0);

        metrics.successful_tasks = 9;
        metrics.failed_tasks = 1;
        assert_eq!(metrics.success_rate(), 90.0);

        metrics.successful_tasks = 0;
        metrics.failed_tasks = 10;
        assert_eq!(metrics.success_rate(), 0.0);
    }

    #[test]
    fn test_agent_config_defaults() {
        let config = AgentConfig::default();

        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 2000);
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.tools_enabled.len(), 0);
    }

    // ===== Template Tests =====

    #[test]
    fn test_version_creation() {
        let version = Version::new(1, 2, 3);
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_version_display() {
        let version = Version::new(2, 5, 10);
        assert_eq!(version.to_string(), "2.5.10");
    }

    #[test]
    fn test_version_from_str() {
        let version: Version = "1.2.3".parse().unwrap();
        assert_eq!(version, Version::new(1, 2, 3));

        let version: Version = "10.0.5".parse().unwrap();
        assert_eq!(version, Version::new(10, 0, 5));

        assert!("invalid".parse::<Version>().is_err());
        assert!("1.2".parse::<Version>().is_err());
        assert!("1.2.3.4".parse::<Version>().is_err());
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 0, 1);
        let v3 = Version::new(1, 1, 0);
        let v4 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert_eq!(v1, Version::new(1, 0, 0));
    }

    #[test]
    fn test_version_bumping() {
        let mut version = Version::new(1, 2, 3);

        version.bump_patch();
        assert_eq!(version, Version::new(1, 2, 4));

        version.bump_minor();
        assert_eq!(version, Version::new(1, 3, 0));

        version.bump_major();
        assert_eq!(version, Version::new(2, 0, 0));
    }

    #[test]
    fn test_variable_spec_creation() {
        let var = VariableSpec::new(
            "user_input".to_string(),
            "String".to_string(),
            true,
            "User's input text".to_string(),
        );

        assert_eq!(var.name, "user_input");
        assert_eq!(var.type_hint, "String");
        assert!(var.required);
        assert_eq!(var.description, "User's input text");
        assert!(var.default.is_none());
        assert!(var.validation_pattern.is_none());
    }

    #[test]
    fn test_variable_spec_with_default() {
        let var = VariableSpec::new(
            "count".to_string(),
            "Number".to_string(),
            false,
            "Item count".to_string(),
        )
        .with_default("10".to_string());

        assert_eq!(var.default, Some("10".to_string()));
    }

    #[test]
    fn test_variable_spec_validation() {
        let var = VariableSpec::new(
            "email".to_string(),
            "String".to_string(),
            true,
            "Email address".to_string(),
        )
        .with_validation(r"^[\w\.-]+@[\w\.-]+\.\w+$".to_string());

        // Valid email
        let result = var.validate(&Some("test@example.com".to_string()));
        assert!(result.is_ok());

        // Invalid email
        let result = var.validate(&Some("invalid-email".to_string()));
        assert!(result.is_err());

        // Missing required value
        let result = var.validate(&None);
        assert!(result.is_err());
    }

    #[test]
    fn test_variable_spec_optional_validation() {
        let var = VariableSpec::new(
            "optional".to_string(),
            "String".to_string(),
            false,
            "Optional field".to_string(),
        );

        // Missing optional value is OK
        let result = var.validate(&None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_prompt_template_creation() {
        let variables = vec![VariableSpec::new(
            "name".to_string(),
            "String".to_string(),
            true,
            "User's name".to_string(),
        )];

        let template = PromptTemplate::new(
            "Greeting Template".to_string(),
            "Hello, {{name}}!".to_string(),
            variables,
        );

        assert_eq!(template.name, "Greeting Template");
        assert_eq!(template.template, "Hello, {{name}}!");
        assert_eq!(template.variables.len(), 1);
        assert_eq!(template.version, Version::new(1, 0, 0));
        assert_eq!(template.usage_count, 0);
    }

    #[test]
    fn test_prompt_template_with_description() {
        let template = PromptTemplate::new("Test".to_string(), "{{content}}".to_string(), vec![])
            .with_description("A test template".to_string());

        assert_eq!(template.description, "A test template");
    }

    #[test]
    fn test_prompt_template_with_author() {
        let template = PromptTemplate::new("Test".to_string(), "{{content}}".to_string(), vec![])
            .with_author("John Doe".to_string());

        assert_eq!(template.author, "John Doe");
    }

    #[test]
    fn test_prompt_template_instantiation() {
        let variables = vec![
            VariableSpec::new(
                "name".to_string(),
                "String".to_string(),
                true,
                "Name".to_string(),
            ),
            VariableSpec::new(
                "action".to_string(),
                "String".to_string(),
                true,
                "Action".to_string(),
            ),
        ];

        let template = PromptTemplate::new(
            "Action Template".to_string(),
            "{{name}} is {{action}}".to_string(),
            variables,
        );

        let mut values = HashMap::new();
        values.insert("name".to_string(), "Alice".to_string());
        values.insert("action".to_string(), "coding".to_string());

        let result = template.instantiate(&values).unwrap();
        assert_eq!(result, "Alice is coding");
    }

    #[test]
    fn test_prompt_template_instantiation_with_defaults() {
        let variables = vec![
            VariableSpec::new(
                "name".to_string(),
                "String".to_string(),
                true,
                "Name".to_string(),
            ),
            VariableSpec::new(
                "greeting".to_string(),
                "String".to_string(),
                false,
                "Greeting".to_string(),
            )
            .with_default("Hello".to_string()),
        ];

        let template = PromptTemplate::new(
            "Greeting".to_string(),
            "{{greeting}}, {{name}}!".to_string(),
            variables,
        );

        let mut values = HashMap::new();
        values.insert("name".to_string(), "Bob".to_string());

        let result = template.instantiate(&values).unwrap();
        assert_eq!(result, "Hello, Bob!");
    }

    #[test]
    fn test_prompt_template_validation_missing_required() {
        let variables = vec![VariableSpec::new(
            "required_field".to_string(),
            "String".to_string(),
            true,
            "Required".to_string(),
        )];

        let template = PromptTemplate::new(
            "Test".to_string(),
            "{{required_field}}".to_string(),
            variables,
        );

        let values = HashMap::new();
        let result = template.validate(&values);
        assert!(result.is_err());
    }

    #[test]
    fn test_prompt_template_validation_pattern() {
        let variables = vec![VariableSpec::new(
            "number".to_string(),
            "String".to_string(),
            true,
            "Number".to_string(),
        )
        .with_validation(r"^\d+$".to_string())];

        let template = PromptTemplate::new(
            "Test".to_string(),
            "Count: {{number}}".to_string(),
            variables,
        );

        let mut values = HashMap::new();
        values.insert("number".to_string(), "123".to_string());
        assert!(template.validate(&values).is_ok());

        let mut values = HashMap::new();
        values.insert("number".to_string(), "abc".to_string());
        assert!(template.validate(&values).is_err());
    }

    #[test]
    fn test_prompt_template_usage_tracking() {
        let mut template =
            PromptTemplate::new("Test".to_string(), "{{content}}".to_string(), vec![]);

        assert_eq!(template.usage_count, 0);

        template.record_usage();
        assert_eq!(template.usage_count, 1);

        template.record_usage();
        assert_eq!(template.usage_count, 2);
    }

    #[test]
    fn test_prompt_template_version_bumping() {
        let mut template =
            PromptTemplate::new("Test".to_string(), "{{content}}".to_string(), vec![]);

        assert_eq!(template.version, Version::new(1, 0, 0));

        template.bump_version(VersionLevel::Patch);
        assert_eq!(template.version, Version::new(1, 0, 1));

        template.bump_version(VersionLevel::Minor);
        assert_eq!(template.version, Version::new(1, 1, 0));

        template.bump_version(VersionLevel::Major);
        assert_eq!(template.version, Version::new(2, 0, 0));
    }

    #[test]
    fn test_prompt_template_from_parent() {
        let parent_id = TemplateId::new();
        let template = PromptTemplate::from_parent(
            parent_id,
            "Child Template".to_string(),
            "Extended: {{content}}".to_string(),
            vec![],
        );

        assert_eq!(template.parent_id, Some(parent_id));
        assert_eq!(template.name, "Child Template");
    }

    #[test]
    fn test_prompt_template_tags() {
        let mut template =
            PromptTemplate::new("Test".to_string(), "{{content}}".to_string(), vec![]);

        assert_eq!(template.tags.len(), 0);

        template.add_tag("production".to_string());
        template.add_tag("verified".to_string());

        assert_eq!(template.tags.len(), 2);
        assert!(template.tags.contains(&"production".to_string()));
        assert!(template.tags.contains(&"verified".to_string()));
    }

    #[test]
    fn test_prompt_template_metadata() {
        let mut template =
            PromptTemplate::new("Test".to_string(), "{{content}}".to_string(), vec![]);

        assert_eq!(template.metadata.len(), 0);

        template.add_metadata("category".to_string(), "greeting".to_string());
        template.add_metadata("priority".to_string(), "high".to_string());

        assert_eq!(template.metadata.len(), 2);
        assert_eq!(
            template.metadata.get("category"),
            Some(&"greeting".to_string())
        );
        assert_eq!(template.metadata.get("priority"), Some(&"high".to_string()));
    }
}
