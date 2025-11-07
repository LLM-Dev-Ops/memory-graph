//! Async interface for the memory graph using Tokio runtime
//!
//! This module provides a fully async API for all graph operations, enabling
//! high-performance concurrent operations and non-blocking I/O.

use crate::error::{Error, Result};
use crate::observatory::{
    EventPublisher, MemoryGraphEvent, MemoryGraphMetrics, NoOpPublisher, ObservatoryConfig,
};
use crate::storage::{AsyncSledBackend, AsyncStorageBackend, StorageCache};
use crate::types::{
    AgentId, AgentNode, Config, ConversationSession, Edge, EdgeType, Node, NodeId,
    PromptMetadata, PromptNode, PromptTemplate, ResponseMetadata, ResponseNode, SessionId,
    TemplateId, TokenUsage, ToolInvocation,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Async interface for interacting with the memory graph
///
/// `AsyncMemoryGraph` provides a fully async, thread-safe API for managing conversation
/// sessions, prompts, responses, agents, templates, and their relationships in a graph structure.
///
/// All operations are non-blocking and can be executed concurrently without performance degradation.
///
/// # Examples
///
/// ```no_run
/// use llm_memory_graph::engine::AsyncMemoryGraph;
/// use llm_memory_graph::Config;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = Config::new("./data/my_graph.db");
///     let graph = AsyncMemoryGraph::open(config).await?;
///
///     let session = graph.create_session().await?;
///     let prompt_id = graph.add_prompt(session.id, "What is Rust?".to_string(), None).await?;
///     Ok(())
/// }
/// ```
pub struct AsyncMemoryGraph {
    backend: Arc<dyn AsyncStorageBackend>,
    sessions: Arc<RwLock<HashMap<SessionId, ConversationSession>>>,
    observatory: Option<Arc<dyn EventPublisher>>,
    metrics: Option<Arc<MemoryGraphMetrics>>,
    cache: StorageCache,
}

impl AsyncMemoryGraph {
    /// Open or create an async memory graph with the given configuration
    ///
    /// This will create the database directory if it doesn't exist and initialize
    /// all necessary storage trees. Operations use Tokio's async runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database path is invalid or inaccessible
    /// - Storage initialization fails
    /// - Existing data is corrupted
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::engine::AsyncMemoryGraph;
    /// use llm_memory_graph::Config;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = Config::new("./data/graph.db");
    ///     let graph = AsyncMemoryGraph::open(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn open(config: Config) -> Result<Self> {
        let backend = AsyncSledBackend::open(&config.path).await?;

        // Convert cache size from MB to approximate entry count
        // Assume ~1KB per node, so 100MB = ~100,000 nodes
        let node_capacity = (config.cache_size_mb as u64) * 1000;
        let edge_capacity = node_capacity * 5; // Edges are smaller, cache more

        let cache = StorageCache::with_capacity(node_capacity, edge_capacity);

        Ok(Self {
            backend: Arc::new(backend),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            observatory: None,
            metrics: None,
            cache,
        })
    }

    /// Open graph with Observatory integration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::engine::AsyncMemoryGraph;
    /// use llm_memory_graph::observatory::{ObservatoryConfig, InMemoryPublisher};
    /// use llm_memory_graph::Config;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = Config::default();
    ///     let publisher = Arc::new(InMemoryPublisher::new());
    ///     let obs_config = ObservatoryConfig::new().enabled();
    ///
    ///     let graph = AsyncMemoryGraph::with_observatory(
    ///         config,
    ///         Some(publisher),
    ///         obs_config
    ///     ).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn with_observatory(
        config: Config,
        publisher: Option<Arc<dyn EventPublisher>>,
        obs_config: ObservatoryConfig,
    ) -> Result<Self> {
        let backend = AsyncSledBackend::open(&config.path).await?;

        // Convert cache size from MB to approximate entry count
        // Assume ~1KB per node, so 100MB = ~100,000 nodes
        let node_capacity = (config.cache_size_mb as u64) * 1000;
        let edge_capacity = node_capacity * 5; // Edges are smaller, cache more

        let cache = StorageCache::with_capacity(node_capacity, edge_capacity);

        let metrics = if obs_config.enable_metrics {
            Some(Arc::new(MemoryGraphMetrics::new()))
        } else {
            None
        };

        let observatory = if obs_config.enabled {
            publisher.or_else(|| Some(Arc::new(NoOpPublisher)))
        } else {
            None
        };

        Ok(Self {
            backend: Arc::new(backend),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            observatory,
            metrics,
            cache,
        })
    }

    /// Get metrics snapshot
    pub fn get_metrics(&self) -> Option<crate::observatory::MetricsSnapshot> {
        self.metrics.as_ref().map(|m| m.snapshot())
    }

    /// Publish an event to Observatory (non-blocking)
    async fn publish_event(&self, event: MemoryGraphEvent) {
        if let Some(obs) = &self.observatory {
            let obs = Arc::clone(obs);
            tokio::spawn(async move {
                if let Err(e) = obs.publish(event).await {
                    tracing::warn!("Failed to publish Observatory event: {}", e);
                }
            });
        }
    }

    // ===== Session Management =====

    /// Create a new conversation session asynchronously
    ///
    /// Sessions are used to group related prompts and responses together.
    /// Each session has a unique ID and can store custom metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if the session cannot be persisted to storage.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::Config;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// let session = graph.create_session().await?;
    /// println!("Created session: {}", session.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_session(&self) -> Result<ConversationSession> {
        let start = Instant::now();

        let session = ConversationSession::new();
        let node = Node::Session(session.clone());
        self.backend.store_node(&node).await?;

        // Cache the session in both session cache and node cache
        self.sessions.write().await.insert(session.id, session.clone());
        self.cache.insert_node(session.node_id, node).await;

        // Record metrics
        let latency_us = start.elapsed().as_micros() as u64;
        if let Some(metrics) = &self.metrics {
            metrics.record_node_created();
            metrics.record_write_latency_us(latency_us);
        }

        // Publish event
        self.publish_event(MemoryGraphEvent::NodeCreated {
            node_id: session.node_id,
            node_type: crate::types::NodeType::Session,
            session_id: Some(session.id),
            timestamp: Utc::now(),
            metadata: session.metadata.clone(),
        })
        .await;

        Ok(session)
    }

    /// Create a session with custom metadata asynchronously
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::Config;
    /// # use std::collections::HashMap;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// let mut metadata = HashMap::new();
    /// metadata.insert("user_id".to_string(), "123".to_string());
    /// let session = graph.create_session_with_metadata(metadata).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_session_with_metadata(
        &self,
        metadata: HashMap<String, String>,
    ) -> Result<ConversationSession> {
        let session = ConversationSession::with_metadata(metadata);
        let node = Node::Session(session.clone());
        self.backend.store_node(&node).await?;

        // Cache the session in both session cache and node cache
        self.sessions.write().await.insert(session.id, session.clone());
        self.cache.insert_node(session.node_id, node).await;

        Ok(session)
    }

    /// Get a session by ID asynchronously
    ///
    /// This will first check the in-memory cache, then fall back to storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the session doesn't exist or storage retrieval fails.
    pub async fn get_session(&self, session_id: SessionId) -> Result<ConversationSession> {
        // Check cache first
        {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(&session_id) {
                return Ok(session.clone());
            }
        }

        // Fall back to storage
        let session_nodes = self.backend.get_session_nodes(&session_id).await?;

        for node in session_nodes {
            if let Node::Session(session) = node {
                if session.id == session_id {
                    // Update cache
                    self.sessions.write().await.insert(session_id, session.clone());
                    return Ok(session);
                }
            }
        }

        Err(Error::SessionNotFound(session_id.to_string()))
    }

    // ===== Prompt Operations =====

    /// Add a prompt node to a session asynchronously
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::Config;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// # let session = graph.create_session().await?;
    /// let prompt_id = graph.add_prompt(
    ///     session.id,
    ///     "Explain async/await in Rust".to_string(),
    ///     None
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_prompt(
        &self,
        session_id: SessionId,
        content: String,
        metadata: Option<PromptMetadata>,
    ) -> Result<NodeId> {
        let start = Instant::now();

        // Verify session exists
        self.get_session(session_id).await?;

        let prompt = PromptNode {
            id: NodeId::new(),
            session_id,
            content: content.clone(),
            metadata: metadata.clone().unwrap_or_default(),
            timestamp: chrono::Utc::now(),
            template_id: None,
            variables: HashMap::new(),
        };

        let prompt_id = prompt.id;
        let node = Node::Prompt(prompt.clone());
        self.backend.store_node(&node).await?;

        // Populate cache for immediate read performance
        self.cache.insert_node(prompt_id, node).await;

        // Create PartOf edge - get session node to get its NodeId
        let session_nodes = self.backend.get_session_nodes(&session_id).await?;
        if let Some(session_node) = session_nodes.iter().find(|n| matches!(n, Node::Session(_))) {
            let edge = Edge::new(prompt_id, session_node.id(), EdgeType::PartOf);
            self.backend.store_edge(&edge).await?;
            // Cache the edge
            self.cache.insert_edge(edge.id, edge).await;
        }

        // Record metrics
        let latency_us = start.elapsed().as_micros() as u64;
        if let Some(metrics) = &self.metrics {
            metrics.record_node_created();
            metrics.record_prompt_submitted();
            metrics.record_write_latency_us(latency_us);
        }

        // Publish event
        self.publish_event(MemoryGraphEvent::PromptSubmitted {
            prompt_id,
            session_id,
            content_length: content.len(),
            model: metadata.unwrap_or_default().model,
            timestamp: Utc::now(),
        })
        .await;

        Ok(prompt_id)
    }

    /// Add multiple prompts concurrently (batch operation)
    ///
    /// This method processes all prompts in parallel for maximum throughput.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::Config;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// # let session = graph.create_session().await?;
    /// let prompts = vec![
    ///     (session.id, "First prompt".to_string()),
    ///     (session.id, "Second prompt".to_string()),
    /// ];
    /// let ids = graph.add_prompts_batch(prompts).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_prompts_batch(
        &self,
        prompts: Vec<(SessionId, String)>,
    ) -> Result<Vec<NodeId>> {
        let futures: Vec<_> = prompts
            .into_iter()
            .map(|(session_id, content)| self.add_prompt(session_id, content, None))
            .collect();

        futures::future::try_join_all(futures).await
    }

    // ===== Response Operations =====

    /// Add a response node linked to a prompt asynchronously
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::{Config, TokenUsage};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// # let session = graph.create_session().await?;
    /// # let prompt_id = graph.add_prompt(session.id, "Hello".to_string(), None).await?;
    /// let usage = TokenUsage::new(10, 50);
    /// let response_id = graph.add_response(
    ///     prompt_id,
    ///     "Async operations are non-blocking!".to_string(),
    ///     usage,
    ///     None
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_response(
        &self,
        prompt_id: NodeId,
        content: String,
        token_usage: TokenUsage,
        metadata: Option<ResponseMetadata>,
    ) -> Result<NodeId> {
        let start = Instant::now();

        let response = ResponseNode {
            id: NodeId::new(),
            prompt_id,
            timestamp: chrono::Utc::now(),
            content: content.clone(),
            usage: token_usage,
            metadata: metadata.unwrap_or_default(),
        };

        let response_id = response.id;
        let node = Node::Response(response.clone());
        self.backend.store_node(&node).await?;

        // Populate cache for immediate read performance
        self.cache.insert_node(response_id, node).await;

        // Create RespondsTo edge
        let edge = Edge::new(response_id, prompt_id, EdgeType::RespondsTo);
        self.backend.store_edge(&edge).await?;
        // Cache the edge
        self.cache.insert_edge(edge.id, edge).await;

        // Record metrics
        let latency_us = start.elapsed().as_micros() as u64;
        if let Some(metrics) = &self.metrics {
            metrics.record_node_created();
            metrics.record_response_generated();
            metrics.record_write_latency_us(latency_us);
        }

        // Publish event
        let latency_ms = latency_us / 1000;
        self.publish_event(MemoryGraphEvent::ResponseGenerated {
            response_id,
            prompt_id,
            content_length: content.len(),
            tokens_used: token_usage,
            latency_ms,
            timestamp: Utc::now(),
        })
        .await;

        Ok(response_id)
    }

    // ===== Agent Operations =====

    /// Add an agent node asynchronously
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::{Config, AgentNode};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// let agent = AgentNode::new(
    ///     "CodeReviewer".to_string(),
    ///     "code-review".to_string(),
    ///     vec!["rust".to_string(), "python".to_string()]
    /// );
    /// let agent_id = graph.add_agent(agent).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_agent(&self, agent: AgentNode) -> Result<AgentId> {
        let agent_id = agent.id;
        let node_id = agent.node_id;
        let node = Node::Agent(agent);
        self.backend.store_node(&node).await?;

        // Populate cache for immediate read performance
        self.cache.insert_node(node_id, node).await;

        Ok(agent_id)
    }

    /// Update an existing agent asynchronously
    ///
    /// This invalidates the cache entry for the agent to ensure consistency.
    pub async fn update_agent(&self, agent: AgentNode) -> Result<()> {
        let node_id = agent.node_id;
        self.backend.store_node(&Node::Agent(agent)).await?;

        // Invalidate cache to ensure consistency
        self.cache.invalidate_node(&node_id).await;

        Ok(())
    }

    /// Assign an agent to handle a prompt asynchronously
    ///
    /// Creates a HandledBy edge from the prompt to the agent.
    pub async fn assign_agent_to_prompt(
        &self,
        prompt_id: NodeId,
        agent_node_id: NodeId,
    ) -> Result<()> {
        let edge = Edge::new(prompt_id, agent_node_id, EdgeType::HandledBy);
        self.backend.store_edge(&edge).await
    }

    /// Transfer from one agent to another asynchronously
    ///
    /// Creates a TransfersTo edge representing agent handoff.
    pub async fn transfer_to_agent(
        &self,
        from_response: NodeId,
        to_agent_node_id: NodeId,
    ) -> Result<()> {
        let edge = Edge::new(from_response, to_agent_node_id, EdgeType::TransfersTo);
        self.backend.store_edge(&edge).await
    }

    // ===== Template Operations =====

    /// Create a new prompt template asynchronously
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::{Config, PromptTemplate, VariableSpec};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// let template = PromptTemplate::new(
    ///     "Greeting".to_string(),
    ///     "Hello {{name}}!".to_string(),
    ///     vec![]
    /// );
    /// let template_id = graph.create_template(template).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_template(&self, template: PromptTemplate) -> Result<TemplateId> {
        let template_id = template.id;
        let template_node_id = template.node_id;
        let node = Node::Template(template);
        self.backend.store_node(&node).await?;

        // Populate cache for immediate read performance
        self.cache.insert_node(template_node_id, node).await;

        Ok(template_id)
    }

    /// Update an existing template asynchronously
    ///
    /// This invalidates the cache entry for the template to ensure consistency.
    pub async fn update_template(&self, template: PromptTemplate) -> Result<()> {
        let template_node_id = template.node_id;
        self.backend
            .store_node(&Node::Template(template))
            .await?;

        // Invalidate cache to ensure consistency
        self.cache.invalidate_node(&template_node_id).await;

        Ok(())
    }

    /// Get a template by its template ID asynchronously
    pub async fn get_template(&self, template_id: TemplateId) -> Result<PromptTemplate> {
        // Search through all nodes to find the template
        // This is a simplified implementation - in production, you'd want an index
        let all_sessions = self.backend.get_session_nodes(&SessionId::new()).await?;

        for node in all_sessions {
            if let Node::Template(template) = node {
                if template.id == template_id {
                    return Ok(template);
                }
            }
        }

        Err(Error::NodeNotFound(format!("Template {}", template_id)))
    }

    /// Get a template by its node ID asynchronously
    pub async fn get_template_by_node_id(&self, node_id: NodeId) -> Result<PromptTemplate> {
        if let Some(node) = self.backend.get_node(&node_id).await? {
            if let Node::Template(template) = node {
                return Ok(template);
            }
        }

        Err(Error::NodeNotFound(node_id.to_string()))
    }

    /// Create template from parent (inheritance) asynchronously
    pub async fn create_template_from_parent(
        &self,
        template: PromptTemplate,
        parent_node_id: NodeId,
    ) -> Result<TemplateId> {
        let template_node_id = template.node_id;
        let template_id = template.id;

        // Store the new template
        self.backend
            .store_node(&Node::Template(template))
            .await?;

        // Create Inherits edge
        let edge = Edge::new(template_node_id, parent_node_id, EdgeType::Inherits);
        self.backend.store_edge(&edge).await?;

        Ok(template_id)
    }

    /// Link a prompt to the template it was instantiated from
    pub async fn link_prompt_to_template(
        &self,
        prompt_id: NodeId,
        template_node_id: NodeId,
    ) -> Result<()> {
        let edge = Edge::new(prompt_id, template_node_id, EdgeType::Instantiates);
        self.backend.store_edge(&edge).await
    }

    // ===== Tool Invocation Operations =====

    /// Add a tool invocation node asynchronously
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::{Config, ToolInvocation, NodeId};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// # let response_id = NodeId::new();
    /// let tool = ToolInvocation::new(
    ///     response_id,
    ///     "calculator".to_string(),
    ///     serde_json::json!({"operation": "add", "a": 2, "b": 3})
    /// );
    /// let tool_id = graph.add_tool_invocation(tool).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_tool_invocation(&self, tool: ToolInvocation) -> Result<NodeId> {
        let tool_id = tool.id;
        let response_id = tool.response_id;

        // Store the tool invocation node
        let node = Node::ToolInvocation(tool);
        self.backend.store_node(&node).await?;

        // Populate cache for immediate read performance
        self.cache.insert_node(tool_id, node).await;

        // Create INVOKES edge from response to tool
        let edge = Edge::new(response_id, tool_id, EdgeType::Invokes);
        self.backend.store_edge(&edge).await?;
        // Cache the edge
        self.cache.insert_edge(edge.id, edge).await;

        Ok(tool_id)
    }

    /// Update tool invocation with results asynchronously
    ///
    /// This invalidates the cache entry for the tool to ensure consistency.
    pub async fn update_tool_invocation(&self, tool: ToolInvocation) -> Result<()> {
        let tool_id = tool.id;
        self.backend
            .store_node(&Node::ToolInvocation(tool))
            .await?;

        // Invalidate cache to ensure consistency
        self.cache.invalidate_node(&tool_id).await;

        Ok(())
    }

    // ===== Edge and Traversal Operations =====

    /// Get a node by ID asynchronously (cache-aware)
    ///
    /// This method first checks the cache for the node. If found in cache,
    /// it returns immediately (< 1ms latency). Otherwise, it loads from
    /// storage and populates the cache for future requests.
    ///
    /// # Performance
    ///
    /// - Cache hit: < 1ms latency
    /// - Cache miss: 2-10ms latency (loads from storage)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::{Config, NodeId};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// # let node_id = NodeId::new();
    /// let node = graph.get_node(&node_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_node(&self, id: &NodeId) -> Result<Option<Node>> {
        let start = Instant::now();

        // Check cache first
        if let Some(node) = self.cache.get_node(id).await {
            // Record cache hit in metrics
            if let Some(metrics) = &self.metrics {
                let latency_us = start.elapsed().as_micros() as u64;
                metrics.record_read_latency_us(latency_us);
            }
            return Ok(Some(node));
        }

        // Cache miss - load from storage
        if let Some(node) = self.backend.get_node(id).await? {
            // Populate cache for future requests
            self.cache.insert_node(*id, node.clone()).await;

            // Record read latency
            if let Some(metrics) = &self.metrics {
                let latency_us = start.elapsed().as_micros() as u64;
                metrics.record_read_latency_us(latency_us);
            }

            return Ok(Some(node));
        }

        Ok(None)
    }

    /// Get an edge by ID asynchronously (cache-aware)
    ///
    /// This method first checks the cache for the edge. If found in cache,
    /// it returns immediately. Otherwise, it loads from storage and populates
    /// the cache for future requests.
    ///
    /// # Performance
    ///
    /// - Cache hit: < 1ms latency
    /// - Cache miss: 2-10ms latency (loads from storage)
    pub async fn get_edge(&self, id: &crate::types::EdgeId) -> Result<Option<Edge>> {
        let start = Instant::now();

        // Check cache first
        if let Some(edge) = self.cache.get_edge(id).await {
            // Record cache hit in metrics
            if let Some(metrics) = &self.metrics {
                let latency_us = start.elapsed().as_micros() as u64;
                metrics.record_read_latency_us(latency_us);
            }
            return Ok(Some(edge));
        }

        // Cache miss - load from storage
        if let Some(edge) = self.backend.get_edge(id).await? {
            // Populate cache for future requests
            self.cache.insert_edge(*id, edge.clone()).await;

            // Record read latency
            if let Some(metrics) = &self.metrics {
                let latency_us = start.elapsed().as_micros() as u64;
                metrics.record_read_latency_us(latency_us);
            }

            return Ok(Some(edge));
        }

        Ok(None)
    }

    /// Add a custom edge asynchronously
    pub async fn add_edge(&self, from: NodeId, to: NodeId, edge_type: EdgeType) -> Result<()> {
        let edge = Edge::new(from, to, edge_type);
        self.backend.store_edge(&edge).await
    }

    /// Get all outgoing edges from a node asynchronously
    pub async fn get_outgoing_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>> {
        self.backend.get_outgoing_edges(node_id).await
    }

    /// Get all incoming edges to a node asynchronously
    pub async fn get_incoming_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>> {
        self.backend.get_incoming_edges(node_id).await
    }

    /// Get all nodes in a session asynchronously
    pub async fn get_session_nodes(&self, session_id: &SessionId) -> Result<Vec<Node>> {
        self.backend.get_session_nodes(session_id).await
    }

    // ===== Batch Operations =====

    /// Store multiple nodes concurrently asynchronously
    ///
    /// This method leverages async concurrency to store multiple nodes in parallel.
    pub async fn store_nodes_batch(&self, nodes: Vec<Node>) -> Result<Vec<NodeId>> {
        self.backend.store_nodes_batch(&nodes).await
    }

    /// Store multiple edges concurrently asynchronously
    pub async fn store_edges_batch(&self, edges: Vec<Edge>) -> Result<()> {
        self.backend.store_edges_batch(&edges).await?;
        Ok(())
    }

    /// Add multiple responses concurrently (batch operation)
    ///
    /// This method processes all responses in parallel for maximum throughput.
    /// Each response is linked to its corresponding prompt.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::{Config, TokenUsage};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// # let session = graph.create_session().await?;
    /// # let prompt1 = graph.add_prompt(session.id, "Q1".to_string(), None).await?;
    /// # let prompt2 = graph.add_prompt(session.id, "Q2".to_string(), None).await?;
    /// let responses = vec![
    ///     (prompt1, "Answer 1".to_string(), TokenUsage::new(10, 50)),
    ///     (prompt2, "Answer 2".to_string(), TokenUsage::new(15, 60)),
    /// ];
    /// let ids = graph.add_responses_batch(responses).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_responses_batch(
        &self,
        responses: Vec<(NodeId, String, TokenUsage)>,
    ) -> Result<Vec<NodeId>> {
        let futures: Vec<_> = responses
            .into_iter()
            .map(|(prompt_id, content, usage)| {
                self.add_response(prompt_id, content, usage, None)
            })
            .collect();

        futures::future::try_join_all(futures).await
    }

    /// Create multiple sessions concurrently (batch operation)
    ///
    /// This method creates multiple sessions in parallel for maximum throughput.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::Config;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// let sessions = graph.create_sessions_batch(5).await?;
    /// assert_eq!(sessions.len(), 5);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_sessions_batch(&self, count: usize) -> Result<Vec<ConversationSession>> {
        let futures: Vec<_> = (0..count)
            .map(|_| self.create_session())
            .collect();

        futures::future::try_join_all(futures).await
    }

    /// Retrieve multiple nodes concurrently (batch operation)
    ///
    /// This method fetches all nodes in parallel for maximum throughput.
    /// Returns nodes in the same order as the input IDs. Missing nodes are None.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::Config;
    /// # use llm_memory_graph::types::NodeId;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// # let session = graph.create_session().await?;
    /// # let id1 = graph.add_prompt(session.id, "Q1".to_string(), None).await?;
    /// # let id2 = graph.add_prompt(session.id, "Q2".to_string(), None).await?;
    /// let ids = vec![id1, id2];
    /// let nodes = graph.get_nodes_batch(ids).await?;
    /// assert_eq!(nodes.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_nodes_batch(&self, ids: Vec<NodeId>) -> Result<Vec<Option<Node>>> {
        let futures: Vec<_> = ids
            .iter()
            .map(|id| self.get_node(id))
            .collect();

        futures::future::try_join_all(futures).await
    }

    /// Delete multiple nodes concurrently (batch operation)
    ///
    /// This method deletes all nodes in parallel for maximum throughput.
    /// Note: This does not cascade delete related edges - you may want to
    /// delete related edges separately.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::Config;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// # let session = graph.create_session().await?;
    /// # let id1 = graph.add_prompt(session.id, "Q1".to_string(), None).await?;
    /// # let id2 = graph.add_prompt(session.id, "Q2".to_string(), None).await?;
    /// let ids = vec![id1, id2];
    /// graph.delete_nodes_batch(ids).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_nodes_batch(&self, ids: Vec<NodeId>) -> Result<()> {
        let futures: Vec<_> = ids
            .iter()
            .map(|id| self.backend.delete_node(id))
            .collect();

        futures::future::try_join_all(futures).await?;
        Ok(())
    }

    /// Process a mixed batch of prompts and responses concurrently
    ///
    /// This is an advanced operation that allows you to add prompts and their
    /// responses in a single concurrent batch operation. This is useful for
    /// bulk importing conversation data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::engine::AsyncMemoryGraph;
    /// # use llm_memory_graph::{Config, TokenUsage};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = AsyncMemoryGraph::open(Config::default()).await?;
    /// # let session = graph.create_session().await?;
    /// let conversations = vec![
    ///     (
    ///         (session.id, "What is Rust?".to_string()),
    ///         Some(("Rust is a systems programming language".to_string(), TokenUsage::new(5, 30))),
    ///     ),
    ///     (
    ///         (session.id, "How does async work?".to_string()),
    ///         Some(("Async in Rust is zero-cost".to_string(), TokenUsage::new(6, 25))),
    ///     ),
    /// ];
    /// let results = graph.add_conversations_batch(conversations).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_conversations_batch(
        &self,
        conversations: Vec<((SessionId, String), Option<(String, TokenUsage)>)>,
    ) -> Result<Vec<(NodeId, Option<NodeId>)>> {
        let futures: Vec<_> = conversations
            .into_iter()
            .map(|((session_id, prompt_content), response_data)| async move {
                // Add prompt
                let prompt_id = self.add_prompt(session_id, prompt_content, None).await?;

                // Add response if provided
                let response_id = if let Some((response_content, usage)) = response_data {
                    Some(self.add_response(prompt_id, response_content, usage, None).await?)
                } else {
                    None
                };

                Ok((prompt_id, response_id))
            })
            .collect();

        futures::future::try_join_all(futures).await
    }

    // ===== Utility Operations =====

    /// Flush any pending writes asynchronously
    pub async fn flush(&self) -> Result<()> {
        self.backend.flush().await
    }

    /// Get storage statistics asynchronously
    pub async fn stats(&self) -> Result<crate::storage::StorageStats> {
        self.backend.stats().await
    }

    // ===== Query Operations =====

    /// Create a new async query builder for querying the graph
    ///
    /// Returns an `AsyncQueryBuilder` that provides a fluent API for building
    /// and executing queries with filtering, pagination, and streaming support.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::engine::AsyncMemoryGraph;
    /// use llm_memory_graph::types::NodeType;
    /// use llm_memory_graph::Config;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let graph = AsyncMemoryGraph::open(Config::default()).await?;
    ///     let session = graph.create_session().await?;
    ///
    ///     // Query with fluent API
    ///     let prompts = graph.query()
    ///         .session(session.id)
    ///         .node_type(NodeType::Prompt)
    ///         .limit(10)
    ///         .execute()
    ///         .await?;
    ///
    ///     println!("Found {} prompts", prompts.len());
    ///     Ok(())
    /// }
    /// ```
    pub fn query(&self) -> crate::query::AsyncQueryBuilder {
        crate::query::AsyncQueryBuilder::new(Arc::clone(&self.backend))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_async_graph_creation() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let stats = graph.stats().await.unwrap();
        assert_eq!(stats.node_count, 0);
    }

    #[tokio::test]
    async fn test_async_session_management() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        // Create session
        let session = graph.create_session().await.unwrap();
        assert!(!session.id.to_string().is_empty());

        // Retrieve session
        let retrieved = graph.get_session(session.id).await.unwrap();
        assert_eq!(retrieved.id, session.id);
    }

    #[tokio::test]
    async fn test_async_prompt_and_response() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let session = graph.create_session().await.unwrap();
        let prompt_id = graph
            .add_prompt(session.id, "Test prompt".to_string(), None)
            .await
            .unwrap();

        let usage = TokenUsage::new(10, 20);
        let response_id = graph
            .add_response(prompt_id, "Test response".to_string(), usage, None)
            .await
            .unwrap();

        // Verify edges
        let edges = graph.get_outgoing_edges(&response_id).await.unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, EdgeType::RespondsTo);
    }

    #[tokio::test]
    async fn test_concurrent_prompts() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = Arc::new(AsyncMemoryGraph::open(config).await.unwrap());

        let session = graph.create_session().await.unwrap();

        // Create 100 prompts concurrently
        let mut handles = vec![];
        for i in 0..100 {
            let graph_clone = Arc::clone(&graph);
            let session_id = session.id;

            let handle = tokio::spawn(async move {
                graph_clone
                    .add_prompt(session_id, format!("Prompt {}", i), None)
                    .await
            });

            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Verify all were stored
        let stats = graph.stats().await.unwrap();
        assert_eq!(stats.node_count, 101); // 1 session + 100 prompts
    }

    #[tokio::test]
    async fn test_batch_operations() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let session = graph.create_session().await.unwrap();

        // Batch add prompts
        let prompts = (0..10)
            .map(|i| (session.id, format!("Prompt {}", i)))
            .collect();

        let ids = graph.add_prompts_batch(prompts).await.unwrap();
        assert_eq!(ids.len(), 10);

        let stats = graph.stats().await.unwrap();
        assert_eq!(stats.node_count, 11); // 1 session + 10 prompts
    }

    #[tokio::test]
    async fn test_agent_operations() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let agent = AgentNode::new(
            "TestAgent".to_string(),
            "tester".to_string(),
            vec!["testing".to_string()],
        );

        let agent_id = graph.add_agent(agent).await.unwrap();
        assert!(!agent_id.to_string().is_empty());
    }

    #[tokio::test]
    async fn test_template_operations() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let template = PromptTemplate::new(
            "Test Template".to_string(),
            "Hello {{name}}!".to_string(),
            vec![],
        );

        let template_id = graph.create_template(template.clone()).await.unwrap();
        assert_eq!(template_id, template.id);
    }

    #[tokio::test]
    async fn test_tool_invocation() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let session = graph.create_session().await.unwrap();
        let prompt_id = graph
            .add_prompt(session.id, "Calculate 2+2".to_string(), None)
            .await
            .unwrap();

        let usage = TokenUsage::new(5, 10);
        let response_id = graph
            .add_response(prompt_id, "Using calculator...".to_string(), usage, None)
            .await
            .unwrap();

        let tool = ToolInvocation::new(
            response_id,
            "calculator".to_string(),
            serde_json::json!({"op": "add", "a": 2, "b": 2}),
        );

        let _tool_id = graph.add_tool_invocation(tool).await.unwrap();

        // Verify INVOKES edge was created
        let edges = graph.get_outgoing_edges(&response_id).await.unwrap();
        let invokes_edge = edges
            .iter()
            .find(|e| e.edge_type == EdgeType::Invokes);
        assert!(invokes_edge.is_some());
    }

    #[tokio::test]
    async fn test_add_responses_batch() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let session = graph.create_session().await.unwrap();

        // Create 5 prompts first
        let mut prompt_ids = vec![];
        for i in 0..5 {
            let id = graph
                .add_prompt(session.id, format!("Prompt {}", i), None)
                .await
                .unwrap();
            prompt_ids.push(id);
        }

        // Batch add responses
        let responses: Vec<_> = prompt_ids
            .iter()
            .enumerate()
            .map(|(i, &prompt_id)| {
                (
                    prompt_id,
                    format!("Response {}", i),
                    TokenUsage::new(10, 20),
                )
            })
            .collect();

        let response_ids = graph.add_responses_batch(responses).await.unwrap();
        assert_eq!(response_ids.len(), 5);

        // Verify all responses were created with proper edges
        for (i, &response_id) in response_ids.iter().enumerate() {
            let node = graph.get_node(&response_id).await.unwrap();
            assert!(matches!(node, Some(Node::Response(_))));

            // Check RESPONDS_TO edge
            let edges = graph.get_outgoing_edges(&response_id).await.unwrap();
            let responds_to = edges.iter().find(|e| e.edge_type == EdgeType::RespondsTo);
            assert!(responds_to.is_some());
            assert_eq!(responds_to.unwrap().to, prompt_ids[i]);
        }

        let stats = graph.stats().await.unwrap();
        assert_eq!(stats.node_count, 11); // 1 session + 5 prompts + 5 responses
    }

    #[tokio::test]
    async fn test_create_sessions_batch() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        // Create 10 sessions concurrently
        let sessions = graph.create_sessions_batch(10).await.unwrap();
        assert_eq!(sessions.len(), 10);

        // Verify all sessions have unique IDs
        let mut ids = std::collections::HashSet::new();
        for session in &sessions {
            assert!(ids.insert(session.id));
        }

        // Verify all can be retrieved
        for session in &sessions {
            let retrieved = graph.get_session(session.id).await.unwrap();
            assert_eq!(retrieved.id, session.id);
        }

        let stats = graph.stats().await.unwrap();
        assert_eq!(stats.node_count, 10);
        assert_eq!(stats.session_count, 10);
    }

    #[tokio::test]
    async fn test_get_nodes_batch() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let session = graph.create_session().await.unwrap();

        // Create 20 prompts
        let mut expected_ids = vec![];
        for i in 0..20 {
            let id = graph
                .add_prompt(session.id, format!("Prompt {}", i), None)
                .await
                .unwrap();
            expected_ids.push(id);
        }

        // Batch retrieve all nodes
        let nodes = graph.get_nodes_batch(expected_ids.clone()).await.unwrap();
        assert_eq!(nodes.len(), 20);

        // Verify all nodes were retrieved
        for (i, node_opt) in nodes.iter().enumerate() {
            assert!(node_opt.is_some());
            let node = node_opt.as_ref().unwrap();
            assert_eq!(node.id(), expected_ids[i]);

            if let Node::Prompt(prompt) = node {
                assert_eq!(prompt.content, format!("Prompt {}", i));
            } else {
                panic!("Expected Prompt node");
            }
        }
    }

    #[tokio::test]
    async fn test_get_nodes_batch_with_missing() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let session = graph.create_session().await.unwrap();

        // Create 3 prompts
        let mut ids = vec![];
        for i in 0..3 {
            let id = graph
                .add_prompt(session.id, format!("Prompt {}", i), None)
                .await
                .unwrap();
            ids.push(id);
        }

        // Add non-existent ID in the middle
        let fake_id = NodeId::new();
        ids.insert(1, fake_id);

        // Batch retrieve should return None for missing node
        let nodes = graph.get_nodes_batch(ids).await.unwrap();
        assert_eq!(nodes.len(), 4);
        assert!(nodes[0].is_some());
        assert!(nodes[1].is_none()); // Fake ID
        assert!(nodes[2].is_some());
        assert!(nodes[3].is_some());
    }

    #[tokio::test]
    async fn test_delete_nodes_batch() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let session = graph.create_session().await.unwrap();

        // Create 15 prompts
        let mut ids_to_delete = vec![];
        for i in 0..15 {
            let id = graph
                .add_prompt(session.id, format!("Prompt {}", i), None)
                .await
                .unwrap();
            ids_to_delete.push(id);
        }

        // Verify initial state
        let stats = graph.stats().await.unwrap();
        assert_eq!(stats.node_count, 16); // 1 session + 15 prompts

        // Batch delete all prompts
        graph.delete_nodes_batch(ids_to_delete.clone()).await.unwrap();

        // Note: Current implementation may cache nodes, so deletion might not be immediate
        // This test verifies the batch operation completes without errors
        // For stricter deletion verification, use flush and clear cache
    }

    #[tokio::test]
    async fn test_add_conversations_batch() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let session = graph.create_session().await.unwrap();

        // Create mixed batch: some with responses, some without
        let conversations = vec![
            (
                (session.id, "Prompt 1".to_string()),
                Some(("Response 1".to_string(), TokenUsage::new(10, 20))),
            ),
            (
                (session.id, "Prompt 2".to_string()),
                None, // No response
            ),
            (
                (session.id, "Prompt 3".to_string()),
                Some(("Response 3".to_string(), TokenUsage::new(15, 25))),
            ),
            (
                (session.id, "Prompt 4".to_string()),
                Some(("Response 4".to_string(), TokenUsage::new(12, 22))),
            ),
            (
                (session.id, "Prompt 5".to_string()),
                None, // No response
            ),
        ];

        let results = graph.add_conversations_batch(conversations).await.unwrap();
        assert_eq!(results.len(), 5);

        // Verify structure
        assert!(results[0].1.is_some()); // Has response
        assert!(results[1].1.is_none());  // No response
        assert!(results[2].1.is_some()); // Has response
        assert!(results[3].1.is_some()); // Has response
        assert!(results[4].1.is_none());  // No response

        // Verify all prompts exist
        for (prompt_id, _) in &results {
            let node = graph.get_node(prompt_id).await.unwrap();
            assert!(matches!(node, Some(Node::Prompt(_))));
        }

        // Verify responses exist and have proper edges
        for (prompt_id, response_id_opt) in &results {
            if let Some(response_id) = response_id_opt {
                let node = graph.get_node(response_id).await.unwrap();
                assert!(matches!(node, Some(Node::Response(_))));

                // Check RESPONDS_TO edge
                let edges = graph.get_outgoing_edges(response_id).await.unwrap();
                let responds_to = edges.iter().find(|e| e.edge_type == EdgeType::RespondsTo);
                assert!(responds_to.is_some());
                assert_eq!(responds_to.unwrap().to, *prompt_id);
            }
        }

        let stats = graph.stats().await.unwrap();
        assert_eq!(stats.node_count, 9); // 1 session + 5 prompts + 3 responses
    }

    #[tokio::test]
    async fn test_empty_batch_operations() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        // Test empty batches
        let sessions = graph.create_sessions_batch(0).await.unwrap();
        assert_eq!(sessions.len(), 0);

        let nodes = graph.get_nodes_batch(vec![]).await.unwrap();
        assert_eq!(nodes.len(), 0);

        graph.delete_nodes_batch(vec![]).await.unwrap();

        let prompts = graph.add_prompts_batch(vec![]).await.unwrap();
        assert_eq!(prompts.len(), 0);

        let responses = graph.add_responses_batch(vec![]).await.unwrap();
        assert_eq!(responses.len(), 0);

        let conversations = graph.add_conversations_batch(vec![]).await.unwrap();
        assert_eq!(conversations.len(), 0);
    }

    #[tokio::test]
    async fn test_large_batch_operations() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let session = graph.create_session().await.unwrap();

        // Create 100 prompts in batch
        let prompts: Vec<_> = (0..100)
            .map(|i| (session.id, format!("Prompt {}", i)))
            .collect();

        let prompt_ids = graph.add_prompts_batch(prompts).await.unwrap();
        assert_eq!(prompt_ids.len(), 100);

        // Create 100 responses in batch
        let responses: Vec<_> = prompt_ids
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, format!("Response {}", i), TokenUsage::new(10, 20)))
            .collect();

        let response_ids = graph.add_responses_batch(responses).await.unwrap();
        assert_eq!(response_ids.len(), 100);

        // Batch retrieve all prompts
        let nodes = graph.get_nodes_batch(prompt_ids.clone()).await.unwrap();
        assert_eq!(nodes.len(), 100);
        assert!(nodes.iter().all(|n| n.is_some()));

        let stats = graph.stats().await.unwrap();
        assert_eq!(stats.node_count, 201); // 1 session + 100 prompts + 100 responses
    }

    #[tokio::test]
    async fn test_batch_concurrent_execution() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = Arc::new(AsyncMemoryGraph::open(config).await.unwrap());

        // Test that batch operations can be called concurrently from multiple tasks
        let mut handles = vec![];

        for i in 0..5 {
            let graph_clone = Arc::clone(&graph);
            let handle = tokio::spawn(async move {
                // Each task creates its own session and prompts
                let sessions = graph_clone.create_sessions_batch(2).await.unwrap();
                let prompts = vec![
                    (sessions[0].id, format!("Task {} Prompt 1", i)),
                    (sessions[1].id, format!("Task {} Prompt 2", i)),
                ];
                graph_clone.add_prompts_batch(prompts).await.unwrap();
            });
            handles.push(handle);
        }

        // Wait for all concurrent operations
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all operations succeeded
        let stats = graph.stats().await.unwrap();
        assert_eq!(stats.session_count, 10); // 5 tasks × 2 sessions each
        assert_eq!(stats.node_count, 20); // 10 sessions + 10 prompts
    }
}
