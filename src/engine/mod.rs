//! Core engine for the memory graph

use crate::error::{Error, Result};
use crate::storage::{SledBackend, StorageBackend};
use crate::types::{
    AgentNode, Config, ConversationSession, Edge, EdgeType, Node, NodeId, PromptMetadata,
    PromptNode, ResponseMetadata, ResponseNode, SessionId, TokenUsage, ToolInvocation,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Main interface for interacting with the memory graph
///
/// `MemoryGraph` provides a thread-safe, high-level API for managing conversation
/// sessions, prompts, responses, and their relationships in a graph structure.
///
/// # Examples
///
/// ```no_run
/// use llm_memory_graph::{MemoryGraph, Config};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::new("./data/my_graph.db");
/// let graph = MemoryGraph::open(config)?;
///
/// let session = graph.create_session()?;
/// let prompt_id = graph.add_prompt(session.id, "What is Rust?".to_string(), None)?;
/// # Ok(())
/// # }
/// ```
pub struct MemoryGraph {
    backend: Arc<dyn StorageBackend>,
    sessions: Arc<RwLock<HashMap<SessionId, ConversationSession>>>,
}

impl MemoryGraph {
    /// Open or create a memory graph with the given configuration
    ///
    /// This will create the database directory if it doesn't exist and initialize
    /// all necessary storage trees.
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
    /// use llm_memory_graph::{MemoryGraph, Config};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config::new("./data/graph.db");
    /// let graph = MemoryGraph::open(config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open(config: Config) -> Result<Self> {
        let backend = SledBackend::open(&config.path)?;

        Ok(Self {
            backend: Arc::new(backend),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create a new conversation session
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
    /// # use llm_memory_graph::{MemoryGraph, Config};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// let session = graph.create_session()?;
    /// println!("Created session: {}", session.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_session(&self) -> Result<ConversationSession> {
        let session = ConversationSession::new();
        self.backend.store_node(&Node::Session(session.clone()))?;

        // Cache the session
        self.sessions
            .write()
            .insert(session.id, session.clone());

        Ok(session)
    }

    /// Create a session with custom metadata
    ///
    /// # Errors
    ///
    /// Returns an error if the session cannot be persisted to storage.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config};
    /// # use std::collections::HashMap;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// let mut metadata = HashMap::new();
    /// metadata.insert("user_id".to_string(), "123".to_string());
    /// let session = graph.create_session_with_metadata(metadata)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_session_with_metadata(
        &self,
        metadata: HashMap<String, String>,
    ) -> Result<ConversationSession> {
        let session = ConversationSession::with_metadata(metadata);
        self.backend.store_node(&Node::Session(session.clone()))?;

        // Cache the session
        self.sessions
            .write()
            .insert(session.id, session.clone());

        Ok(session)
    }

    /// Get a session by ID
    ///
    /// This will first check the in-memory cache, then fall back to storage.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The session doesn't exist
    /// - Storage retrieval fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let created_session = graph.create_session()?;
    /// let session = graph.get_session(created_session.id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_session(&self, session_id: SessionId) -> Result<ConversationSession> {
        // Check cache first
        if let Some(session) = self.sessions.read().get(&session_id) {
            return Ok(session.clone());
        }

        // Fall back to storage
        let nodes = self.backend.get_session_nodes(&session_id)?;
        for node in nodes {
            if let Node::Session(session) = node {
                if session.id == session_id {
                    // Update cache
                    self.sessions
                        .write()
                        .insert(session_id, session.clone());
                    return Ok(session);
                }
            }
        }

        Err(Error::SessionNotFound(session_id.to_string()))
    }

    /// Add a prompt to a session
    ///
    /// This creates a new prompt node and automatically creates edges linking it
    /// to the session and to the previous prompt if one exists.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The session doesn't exist
    /// - Storage operations fail
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// let prompt_id = graph.add_prompt(
    ///     session.id,
    ///     "Explain quantum entanglement".to_string(),
    ///     None,
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_prompt(
        &self,
        session_id: SessionId,
        content: String,
        metadata: Option<PromptMetadata>,
    ) -> Result<NodeId> {
        // Verify session exists
        self.get_session(session_id)?;

        let prompt = if let Some(meta) = metadata {
            PromptNode::with_metadata(session_id, content, meta)
        } else {
            PromptNode::new(session_id, content)
        };

        let prompt_id = prompt.id;
        self.backend.store_node(&Node::Prompt(prompt.clone()))?;

        // Create edge from prompt to session
        let session_nodes = self.backend.get_session_nodes(&session_id)?;
        if let Some(session_node) = session_nodes
            .iter()
            .find(|n| matches!(n, Node::Session(_)))
        {
            let edge = Edge::new(prompt_id, session_node.id(), EdgeType::PartOf);
            self.backend.store_edge(&edge)?;
        }

        // Find the previous prompt in this session and create a Follows edge
        let session_prompts: Vec<_> = session_nodes
            .into_iter()
            .filter_map(|n| {
                if let Node::Prompt(p) = n {
                    Some(p)
                } else {
                    None
                }
            })
            .collect();

        if !session_prompts.is_empty() {
            // Get the most recent prompt (excluding the one we just added)
            let mut previous_prompts: Vec<_> = session_prompts
                .into_iter()
                .filter(|p| p.id != prompt_id)
                .collect();
            previous_prompts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

            if let Some(prev_prompt) = previous_prompts.first() {
                let edge = Edge::new(prompt_id, prev_prompt.id, EdgeType::Follows);
                self.backend.store_edge(&edge)?;
            }
        }

        Ok(prompt_id)
    }

    /// Add a response to a prompt
    ///
    /// This creates a response node and a RespondsTo edge linking it to the prompt.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The prompt doesn't exist
    /// - Storage operations fail
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, TokenUsage};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// let usage = TokenUsage::new(10, 20);
    /// let response_id = graph.add_response(
    ///     prompt_id,
    ///     "Quantum entanglement is...".to_string(),
    ///     usage,
    ///     None,
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_response(
        &self,
        prompt_id: NodeId,
        content: String,
        usage: TokenUsage,
        metadata: Option<ResponseMetadata>,
    ) -> Result<NodeId> {
        // Verify prompt exists
        self.get_node(prompt_id)?;

        let response = if let Some(meta) = metadata {
            ResponseNode::with_metadata(prompt_id, content, usage, meta)
        } else {
            ResponseNode::new(prompt_id, content, usage)
        };

        let response_id = response.id;
        self.backend
            .store_node(&Node::Response(response.clone()))?;

        // Create edge from response to prompt
        let edge = Edge::new(response_id, prompt_id, EdgeType::RespondsTo);
        self.backend.store_edge(&edge)?;

        Ok(response_id)
    }

    /// Add a tool invocation node to the graph
    ///
    /// This creates a tool invocation record and automatically creates an INVOKES edge
    /// from the response to the tool invocation.
    ///
    /// # Arguments
    ///
    /// * `tool` - The tool invocation to add
    ///
    /// # Returns
    ///
    /// The node ID of the created tool invocation
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use llm_memory_graph::*;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// # let response_id = graph.add_response(prompt_id, "Response".to_string(), TokenUsage::new(10, 20), None)?;
    /// let params = serde_json::json!({"operation": "add", "a": 2, "b": 3});
    /// let tool = ToolInvocation::new(response_id, "calculator".to_string(), params);
    /// let tool_id = graph.add_tool_invocation(tool)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_tool_invocation(&self, tool: ToolInvocation) -> Result<NodeId> {
        let tool_id = tool.id;
        let response_id = tool.response_id;

        // Store the tool invocation node
        self.backend.store_node(&Node::ToolInvocation(tool))?;

        // Create INVOKES edge from response to tool
        let edge = Edge::new(response_id, tool_id, EdgeType::Invokes);
        self.backend.store_edge(&edge)?;

        Ok(tool_id)
    }

    /// Update an existing tool invocation with results
    ///
    /// This method updates a tool invocation's status, result, and duration after execution.
    ///
    /// # Arguments
    ///
    /// * `tool_id` - The ID of the tool invocation to update
    /// * `success` - Whether the tool execution was successful
    /// * `result_or_error` - Either the result (if successful) or error message (if failed)
    /// * `duration_ms` - Execution duration in milliseconds
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use llm_memory_graph::*;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// # let response_id = graph.add_response(prompt_id, "Response".to_string(), TokenUsage::new(10, 20), None)?;
    /// # let params = serde_json::json!({"operation": "add", "a": 2, "b": 3});
    /// # let tool = ToolInvocation::new(response_id, "calculator".to_string(), params);
    /// # let tool_id = graph.add_tool_invocation(tool)?;
    /// // Mark tool invocation as successful
    /// let result = serde_json::json!({"result": 5});
    /// graph.update_tool_invocation(tool_id, true, serde_json::to_string(&result)?, 150)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn update_tool_invocation(
        &self,
        tool_id: NodeId,
        success: bool,
        result_or_error: String,
        duration_ms: u64,
    ) -> Result<()> {
        // Get the tool invocation node
        let node = self.backend.get_node(&tool_id)?
            .ok_or_else(|| Error::NodeNotFound(tool_id.to_string()))?;

        if let Node::ToolInvocation(mut tool) = node {
            if success {
                let result: serde_json::Value = serde_json::from_str(&result_or_error)?;
                tool.mark_success(result, duration_ms);
            } else {
                tool.mark_failed(result_or_error, duration_ms);
            }

            // Update the node in storage
            self.backend.store_node(&Node::ToolInvocation(tool))?;
            Ok(())
        } else {
            Err(Error::InvalidNodeType {
                expected: "ToolInvocation".to_string(),
                actual: format!("{:?}", node.node_type()),
            })
        }
    }

    /// Get all tool invocations for a specific response
    ///
    /// # Arguments
    ///
    /// * `response_id` - The response node ID
    ///
    /// # Returns
    ///
    /// A vector of tool invocation nodes
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use llm_memory_graph::*;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// # let response_id = graph.add_response(prompt_id, "Response".to_string(), TokenUsage::new(10, 20), None)?;
    /// let tools = graph.get_response_tools(response_id)?;
    /// println!("Response invoked {} tools", tools.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_response_tools(&self, response_id: NodeId) -> Result<Vec<ToolInvocation>> {
        let edges = self.backend.get_outgoing_edges(&response_id)?;

        let mut tools = Vec::new();
        for edge in edges {
            if edge.edge_type == EdgeType::Invokes {
                if let Some(node) = self.backend.get_node(&edge.to)? {
                    if let Node::ToolInvocation(tool) = node {
                        tools.push(tool);
                    }
                }
            }
        }

        Ok(tools)
    }

    /// Create and register an agent in the graph
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, AgentNode};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// let agent = AgentNode::new(
    ///     "Researcher".to_string(),
    ///     "research".to_string(),
    ///     vec!["web_search".to_string(), "summarize".to_string()],
    /// );
    /// let agent_id = graph.add_agent(agent)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_agent(&self, agent: AgentNode) -> Result<NodeId> {
        let node_id = agent.node_id;
        self.backend.store_node(&Node::Agent(agent))?;
        Ok(node_id)
    }

    /// Update an existing agent's data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The agent doesn't exist
    /// - Storage update fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, AgentNode};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);
    /// # let node_id = graph.add_agent(agent)?;
    /// let node = graph.get_node(node_id)?;
    /// if let llm_memory_graph::types::Node::Agent(mut agent) = node {
    ///     agent.update_metrics(true, 250, 150);
    ///     graph.update_agent(agent)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn update_agent(&self, agent: AgentNode) -> Result<()> {
        self.backend.store_node(&Node::Agent(agent))?;
        Ok(())
    }

    /// Assign an agent to handle a prompt
    ///
    /// Creates a HandledBy edge from the prompt to the agent.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, AgentNode};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// # let agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);
    /// # let agent_node_id = graph.add_agent(agent)?;
    /// graph.assign_agent_to_prompt(prompt_id, agent_node_id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn assign_agent_to_prompt(&self, prompt_id: NodeId, agent_node_id: NodeId) -> Result<()> {
        let edge = Edge::new(prompt_id, agent_node_id, EdgeType::HandledBy);
        self.backend.store_edge(&edge)?;
        Ok(())
    }

    /// Create a transfer from a response to an agent
    ///
    /// Creates a TransfersTo edge indicating agent handoff.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, AgentNode, TokenUsage};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// # let response_id = graph.add_response(prompt_id, "Test".to_string(), TokenUsage::new(10, 10), None)?;
    /// # let agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);
    /// # let agent_node_id = graph.add_agent(agent)?;
    /// graph.transfer_to_agent(response_id, agent_node_id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn transfer_to_agent(&self, response_id: NodeId, agent_node_id: NodeId) -> Result<()> {
        let edge = Edge::new(response_id, agent_node_id, EdgeType::TransfersTo);
        self.backend.store_edge(&edge)?;
        Ok(())
    }

    /// Get the agent assigned to handle a prompt
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No agent is assigned
    /// - Storage retrieval fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, AgentNode};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// # let agent = AgentNode::new("Test".to_string(), "test".to_string(), vec![]);
    /// # let agent_id = graph.add_agent(agent)?;
    /// # graph.assign_agent_to_prompt(prompt_id, agent_id)?;
    /// let agent = graph.get_prompt_agent(prompt_id)?;
    /// println!("Handled by: {}", agent.name);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_prompt_agent(&self, prompt_id: NodeId) -> Result<AgentNode> {
        let edges = self.backend.get_outgoing_edges(&prompt_id)?;
        for edge in edges {
            if edge.edge_type == EdgeType::HandledBy {
                if let Some(node) = self.backend.get_node(&edge.to)? {
                    if let Node::Agent(agent) = node {
                        return Ok(agent);
                    }
                }
            }
        }
        Err(Error::TraversalError(
            "No agent assigned to this prompt".to_string(),
        ))
    }

    /// Get all agents a response was transferred to
    ///
    /// # Errors
    ///
    /// Returns an error if storage retrieval fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, AgentNode, TokenUsage};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// # let response_id = graph.add_response(prompt_id, "Test".to_string(), TokenUsage::new(10, 10), None)?;
    /// let agents = graph.get_agent_handoffs(response_id)?;
    /// for agent in agents {
    ///     println!("Transferred to: {}", agent.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_agent_handoffs(&self, response_id: NodeId) -> Result<Vec<AgentNode>> {
        let edges = self.backend.get_outgoing_edges(&response_id)?;
        let mut agents = Vec::new();
        for edge in edges {
            if edge.edge_type == EdgeType::TransfersTo {
                if let Some(node) = self.backend.get_node(&edge.to)? {
                    if let Node::Agent(agent) = node {
                        agents.push(agent);
                    }
                }
            }
        }
        Ok(agents)
    }

    /// Get a node by its ID
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The node doesn't exist
    /// - Storage retrieval fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// let node = graph.get_node(prompt_id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_node(&self, node_id: NodeId) -> Result<Node> {
        self.backend
            .get_node(&node_id)?
            .ok_or_else(|| Error::NodeNotFound(node_id.to_string()))
    }

    /// Add a custom edge between two nodes
    ///
    /// # Errors
    ///
    /// Returns an error if storage operations fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, EdgeType};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt1 = graph.add_prompt(session.id, "Test1".to_string(), None)?;
    /// # let prompt2 = graph.add_prompt(session.id, "Test2".to_string(), None)?;
    /// graph.add_edge(prompt1, prompt2, EdgeType::Follows)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_edge(&self, from: NodeId, to: NodeId, edge_type: EdgeType) -> Result<()> {
        let edge = Edge::new(from, to, edge_type);
        self.backend.store_edge(&edge)?;
        Ok(())
    }

    /// Get all edges originating from a node
    ///
    /// # Errors
    ///
    /// Returns an error if storage retrieval fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// let edges = graph.get_outgoing_edges(prompt_id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_outgoing_edges(&self, node_id: NodeId) -> Result<Vec<Edge>> {
        self.backend.get_outgoing_edges(&node_id)
    }

    /// Get all edges pointing to a node
    ///
    /// # Errors
    ///
    /// Returns an error if storage retrieval fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// let edges = graph.get_incoming_edges(prompt_id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_incoming_edges(&self, node_id: NodeId) -> Result<Vec<Edge>> {
        self.backend.get_incoming_edges(&node_id)
    }

    /// Get all nodes in a session
    ///
    /// # Errors
    ///
    /// Returns an error if storage retrieval fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// let nodes = graph.get_session_nodes(session.id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_session_nodes(&self, session_id: SessionId) -> Result<Vec<Node>> {
        self.backend.get_session_nodes(&session_id)
    }

    /// Flush all pending writes to disk
    ///
    /// # Errors
    ///
    /// Returns an error if the flush operation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// graph.flush()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn flush(&self) -> Result<()> {
        self.backend.flush()
    }

    /// Get storage statistics
    ///
    /// Returns information about node count, edge count, storage size, etc.
    ///
    /// # Errors
    ///
    /// Returns an error if statistics cannot be retrieved.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// let stats = graph.stats()?;
    /// println!("Nodes: {}, Edges: {}", stats.node_count, stats.edge_count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn stats(&self) -> Result<crate::storage::StorageStats> {
        self.backend.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_graph() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let stats = graph.stats().unwrap();
        assert_eq!(stats.node_count, 0);
    }

    #[test]
    fn test_create_session() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let session = graph.create_session().unwrap();
        let retrieved = graph.get_session(session.id).unwrap();

        assert_eq!(session.id, retrieved.id);
    }

    #[test]
    fn test_add_prompt() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let session = graph.create_session().unwrap();
        let prompt_id = graph
            .add_prompt(session.id, "Test prompt".to_string(), None)
            .unwrap();

        let node = graph.get_node(prompt_id).unwrap();
        assert!(matches!(node, Node::Prompt(_)));
    }

    #[test]
    fn test_add_response() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let session = graph.create_session().unwrap();
        let prompt_id = graph
            .add_prompt(session.id, "Test prompt".to_string(), None)
            .unwrap();

        let usage = TokenUsage::new(10, 20);
        let response_id = graph
            .add_response(prompt_id, "Test response".to_string(), usage, None)
            .unwrap();

        let node = graph.get_node(response_id).unwrap();
        assert!(matches!(node, Node::Response(_)));
    }

    #[test]
    fn test_conversation_chain() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let session = graph.create_session().unwrap();

        // Add first prompt
        let prompt1 = graph
            .add_prompt(session.id, "First prompt".to_string(), None)
            .unwrap();
        let usage1 = TokenUsage::new(5, 10);
        let _response1 = graph
            .add_response(prompt1, "First response".to_string(), usage1, None)
            .unwrap();

        // Add second prompt
        let prompt2 = graph
            .add_prompt(session.id, "Second prompt".to_string(), None)
            .unwrap();
        let usage2 = TokenUsage::new(6, 12);
        let _response2 = graph
            .add_response(prompt2, "Second response".to_string(), usage2, None)
            .unwrap();

        // Verify session has all nodes
        let nodes = graph.get_session_nodes(session.id).unwrap();
        assert!(nodes.len() >= 4); // session + 2 prompts + 2 responses

        // Verify edges exist
        let outgoing = graph.get_outgoing_edges(prompt2).unwrap();
        assert!(!outgoing.is_empty());
    }

    #[test]
    fn test_session_not_found() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let fake_session = SessionId::new();
        let result = graph.get_session(fake_session);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::SessionNotFound(_)));
    }

    #[test]
    fn test_node_not_found() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let fake_node = NodeId::new();
        let result = graph.get_node(fake_node);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NodeNotFound(_)));
    }

    #[test]
    fn test_session_with_metadata() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("user".to_string(), "alice".to_string());

        let session = graph.create_session_with_metadata(metadata).unwrap();
        let retrieved = graph.get_session(session.id).unwrap();

        assert_eq!(
            retrieved.metadata.get("user"),
            Some(&"alice".to_string())
        );
    }
}
