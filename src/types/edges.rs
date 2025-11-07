//! Edge types for connecting nodes in the memory graph
//!
//! This module provides strongly-typed edge definitions with property validation
//! for enterprise-grade graph operations.

use super::{EdgeId, NodeId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of edges that can connect nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    /// Connects sequential prompts in a conversation (Prompt → Prompt)
    Follows,
    /// Links a response to its originating prompt (Response → Prompt)
    RespondsTo,
    /// Tracks which agent handled a prompt (Prompt → Agent)
    HandledBy,
    /// Links a prompt to the session it belongs to (Prompt → Session)
    PartOf,
    /// Links a response to the tools it invoked (Response → ToolInvocation)
    Invokes,
    /// Links a response to the agent it handed off to (Response → Agent)
    TransfersTo,
    /// Links a prompt to the template it was created from (Prompt → Template)
    Instantiates,
    /// Links a template to its parent template (Template → Template)
    Inherits,
    /// Links a prompt to external context sources (Prompt → ExternalContext)
    References,
}

// ===== Edge Property Structs =====

/// Properties for INSTANTIATES edge (Prompt → Template)
///
/// Tracks how a prompt was generated from a template, including the template
/// version and variable bindings used during instantiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstantiatesProperties {
    /// Version of the template that was used
    pub template_version: String,
    /// Variable bindings used during instantiation
    pub variable_bindings: HashMap<String, String>,
    /// When the template was instantiated
    pub instantiation_time: DateTime<Utc>,
}

impl InstantiatesProperties {
    /// Create new instantiation properties
    pub fn new(template_version: String, variable_bindings: HashMap<String, String>) -> Self {
        Self {
            template_version,
            variable_bindings,
            instantiation_time: Utc::now(),
        }
    }

    /// Convert to property map for storage
    pub fn to_properties(&self) -> HashMap<String, String> {
        let mut props = HashMap::new();
        props.insert(
            "template_version".to_string(),
            self.template_version.clone(),
        );
        props.insert(
            "variable_bindings".to_string(),
            serde_json::to_string(&self.variable_bindings).unwrap_or_default(),
        );
        props.insert(
            "instantiation_time".to_string(),
            self.instantiation_time.to_rfc3339(),
        );
        props
    }

    /// Parse from property map
    pub fn from_properties(props: &HashMap<String, String>) -> Result<Self, String> {
        let template_version = props
            .get("template_version")
            .ok_or("Missing template_version")?
            .clone();

        let variable_bindings = props
            .get("variable_bindings")
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let instantiation_time = props
            .get("instantiation_time")
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        Ok(Self {
            template_version,
            variable_bindings,
            instantiation_time,
        })
    }
}

/// Properties for INHERITS edge (Template → Template)
///
/// Tracks template inheritance relationships and modifications made
/// in child templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritsProperties {
    /// Sections that were overridden in the child template
    pub override_sections: Vec<String>,
    /// Semantic diff between parent and child versions
    pub version_diff: String,
    /// Depth of inheritance (1 = direct parent, 2+ = ancestor)
    pub inheritance_depth: u32,
}

impl InheritsProperties {
    /// Create new inheritance properties
    pub fn new(
        override_sections: Vec<String>,
        version_diff: String,
        inheritance_depth: u32,
    ) -> Self {
        Self {
            override_sections,
            version_diff,
            inheritance_depth,
        }
    }

    /// Convert to property map for storage
    pub fn to_properties(&self) -> HashMap<String, String> {
        let mut props = HashMap::new();
        props.insert(
            "override_sections".to_string(),
            serde_json::to_string(&self.override_sections).unwrap_or_default(),
        );
        props.insert("version_diff".to_string(), self.version_diff.clone());
        props.insert(
            "inheritance_depth".to_string(),
            self.inheritance_depth.to_string(),
        );
        props
    }

    /// Parse from property map
    pub fn from_properties(props: &HashMap<String, String>) -> Result<Self, String> {
        let override_sections = props
            .get("override_sections")
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let version_diff = props
            .get("version_diff")
            .ok_or("Missing version_diff")?
            .clone();

        let inheritance_depth = props
            .get("inheritance_depth")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        Ok(Self {
            override_sections,
            version_diff,
            inheritance_depth,
        })
    }
}

/// Properties for INVOKES edge (Response → ToolInvocation)
///
/// Tracks tool invocations made during response generation, including
/// execution order and success status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokesProperties {
    /// Order in which this tool was invoked (0-indexed)
    pub invocation_order: u32,
    /// Whether the tool invocation succeeded
    pub success: bool,
    /// Whether this tool invocation was required for the response
    pub required: bool,
}

impl InvokesProperties {
    /// Create new invocation properties
    pub fn new(invocation_order: u32, success: bool, required: bool) -> Self {
        Self {
            invocation_order,
            success,
            required,
        }
    }

    /// Convert to property map for storage
    pub fn to_properties(&self) -> HashMap<String, String> {
        let mut props = HashMap::new();
        props.insert(
            "invocation_order".to_string(),
            self.invocation_order.to_string(),
        );
        props.insert("success".to_string(), self.success.to_string());
        props.insert("required".to_string(), self.required.to_string());
        props
    }

    /// Parse from property map
    pub fn from_properties(props: &HashMap<String, String>) -> Result<Self, String> {
        let invocation_order = props
            .get("invocation_order")
            .and_then(|s| s.parse().ok())
            .ok_or("Missing or invalid invocation_order")?;

        let success = props
            .get("success")
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);

        let required = props
            .get("required")
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);

        Ok(Self {
            invocation_order,
            success,
            required,
        })
    }
}

/// Priority level for agent handoffs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Priority {
    /// Low priority - can be handled later
    Low,
    /// Normal priority - standard handling
    Normal,
    /// High priority - expedited handling
    High,
    /// Critical priority - immediate attention required
    Critical,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Normal => write!(f, "normal"),
            Priority::High => write!(f, "high"),
            Priority::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Priority::Low),
            "normal" => Ok(Priority::Normal),
            "high" => Ok(Priority::High),
            "critical" => Ok(Priority::Critical),
            _ => Err(format!("Invalid priority: {}", s)),
        }
    }
}

/// Properties for TRANSFERS_TO edge (Response → Agent)
///
/// Tracks agent handoffs, including the reason for transfer and
/// conversation context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransfersToProperties {
    /// Reason for the agent handoff
    pub handoff_reason: String,
    /// Summary of conversation context up to this point
    pub context_summary: String,
    /// Priority level of the handoff
    pub priority: Priority,
}

impl TransfersToProperties {
    /// Create new transfer properties
    pub fn new(handoff_reason: String, context_summary: String, priority: Priority) -> Self {
        Self {
            handoff_reason,
            context_summary,
            priority,
        }
    }

    /// Convert to property map for storage
    pub fn to_properties(&self) -> HashMap<String, String> {
        let mut props = HashMap::new();
        props.insert("handoff_reason".to_string(), self.handoff_reason.clone());
        props.insert("context_summary".to_string(), self.context_summary.clone());
        props.insert("priority".to_string(), self.priority.to_string());
        props
    }

    /// Parse from property map
    pub fn from_properties(props: &HashMap<String, String>) -> Result<Self, String> {
        let handoff_reason = props
            .get("handoff_reason")
            .ok_or("Missing handoff_reason")?
            .clone();

        let context_summary = props
            .get("context_summary")
            .ok_or("Missing context_summary")?
            .clone();

        let priority = props
            .get("priority")
            .and_then(|s| s.parse().ok())
            .unwrap_or(Priority::Normal);

        Ok(Self {
            handoff_reason,
            context_summary,
            priority,
        })
    }
}

/// Type of external context being referenced
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextType {
    /// Document file (PDF, DOCX, etc.)
    Document,
    /// Web page or URL
    WebPage,
    /// Database query result
    Database,
    /// Vector database search result
    VectorSearch,
    /// Previous conversation or memory
    Memory,
}

impl std::fmt::Display for ContextType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextType::Document => write!(f, "document"),
            ContextType::WebPage => write!(f, "webpage"),
            ContextType::Database => write!(f, "database"),
            ContextType::VectorSearch => write!(f, "vector_search"),
            ContextType::Memory => write!(f, "memory"),
        }
    }
}

impl std::str::FromStr for ContextType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "document" => Ok(ContextType::Document),
            "webpage" => Ok(ContextType::WebPage),
            "database" => Ok(ContextType::Database),
            "vector_search" | "vectorsearch" => Ok(ContextType::VectorSearch),
            "memory" => Ok(ContextType::Memory),
            _ => Err(format!("Invalid context type: {}", s)),
        }
    }
}

/// Properties for REFERENCES edge (Prompt → ExternalContext)
///
/// Tracks external context sources used in prompt generation, including
/// relevance scores and specific content references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencesProperties {
    /// Type of external context
    pub context_type: ContextType,
    /// Relevance score (0.0 to 1.0)
    pub relevance_score: f32,
    /// Optional identifier for specific chunk or section
    pub chunk_id: Option<String>,
}

impl ReferencesProperties {
    /// Create new reference properties
    pub fn new(context_type: ContextType, relevance_score: f32, chunk_id: Option<String>) -> Self {
        // Clamp relevance score between 0.0 and 1.0
        let relevance_score = relevance_score.clamp(0.0, 1.0);

        Self {
            context_type,
            relevance_score,
            chunk_id,
        }
    }

    /// Convert to property map for storage
    pub fn to_properties(&self) -> HashMap<String, String> {
        let mut props = HashMap::new();
        props.insert("context_type".to_string(), self.context_type.to_string());
        props.insert(
            "relevance_score".to_string(),
            self.relevance_score.to_string(),
        );
        if let Some(ref chunk_id) = self.chunk_id {
            props.insert("chunk_id".to_string(), chunk_id.clone());
        }
        props
    }

    /// Parse from property map
    pub fn from_properties(props: &HashMap<String, String>) -> Result<Self, String> {
        let context_type = props
            .get("context_type")
            .and_then(|s| s.parse().ok())
            .ok_or("Missing or invalid context_type")?;

        let relevance_score = props
            .get("relevance_score")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0_f32)
            .clamp(0.0, 1.0);

        let chunk_id = props.get("chunk_id").cloned();

        Ok(Self {
            context_type,
            relevance_score,
            chunk_id,
        })
    }
}

/// An edge connecting two nodes in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Unique edge identifier
    pub id: EdgeId,
    /// Source node ID
    pub from: NodeId,
    /// Target node ID
    pub to: NodeId,
    /// Type of relationship
    pub edge_type: EdgeType,
    /// When the edge was created
    pub created_at: DateTime<Utc>,
    /// Additional properties for this edge
    pub properties: HashMap<String, String>,
}

impl Edge {
    /// Create a new edge between two nodes
    #[must_use]
    pub fn new(from: NodeId, to: NodeId, edge_type: EdgeType) -> Self {
        Self {
            id: EdgeId::new(),
            from,
            to,
            edge_type,
            created_at: Utc::now(),
            properties: HashMap::new(),
        }
    }

    /// Create an edge with custom properties
    #[must_use]
    pub fn with_properties(
        from: NodeId,
        to: NodeId,
        edge_type: EdgeType,
        properties: HashMap<String, String>,
    ) -> Self {
        Self {
            id: EdgeId::new(),
            from,
            to,
            edge_type,
            created_at: Utc::now(),
            properties,
        }
    }

    /// Add a property to the edge
    pub fn add_property(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }

    /// Get a property value
    #[must_use]
    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.properties.get(key)
    }

    // ===== Strongly-Typed Edge Builders =====

    /// Create an INSTANTIATES edge with typed properties
    ///
    /// Links a prompt to the template it was instantiated from.
    ///
    /// # Examples
    ///
    /// ```
    /// use llm_memory_graph::types::{Edge, NodeId, InstantiatesProperties};
    /// use std::collections::HashMap;
    ///
    /// let prompt_id = NodeId::new();
    /// let template_id = NodeId::new();
    /// let mut bindings = HashMap::new();
    /// bindings.insert("name".to_string(), "Alice".to_string());
    ///
    /// let properties = InstantiatesProperties::new("1.0.0".to_string(), bindings);
    /// let edge = Edge::instantiates(prompt_id, template_id, properties);
    /// ```
    #[must_use]
    pub fn instantiates(from: NodeId, to: NodeId, properties: InstantiatesProperties) -> Self {
        Self::with_properties(from, to, EdgeType::Instantiates, properties.to_properties())
    }

    /// Create an INHERITS edge with typed properties
    ///
    /// Links a child template to its parent template.
    ///
    /// # Examples
    ///
    /// ```
    /// use llm_memory_graph::types::{Edge, NodeId, InheritsProperties};
    ///
    /// let child_id = NodeId::new();
    /// let parent_id = NodeId::new();
    ///
    /// let properties = InheritsProperties::new(
    ///     vec!["variables".to_string()],
    ///     "Added validation rules".to_string(),
    ///     1,
    /// );
    /// let edge = Edge::inherits(child_id, parent_id, properties);
    /// ```
    #[must_use]
    pub fn inherits(from: NodeId, to: NodeId, properties: InheritsProperties) -> Self {
        Self::with_properties(from, to, EdgeType::Inherits, properties.to_properties())
    }

    /// Create an INVOKES edge with typed properties
    ///
    /// Links a response to a tool it invoked.
    ///
    /// # Examples
    ///
    /// ```
    /// use llm_memory_graph::types::{Edge, NodeId, InvokesProperties};
    ///
    /// let response_id = NodeId::new();
    /// let tool_id = NodeId::new();
    ///
    /// let properties = InvokesProperties::new(0, true, true);
    /// let edge = Edge::invokes(response_id, tool_id, properties);
    /// ```
    #[must_use]
    pub fn invokes(from: NodeId, to: NodeId, properties: InvokesProperties) -> Self {
        Self::with_properties(from, to, EdgeType::Invokes, properties.to_properties())
    }

    /// Create a TRANSFERS_TO edge with typed properties
    ///
    /// Links a response to the agent it transfers control to.
    ///
    /// # Examples
    ///
    /// ```
    /// use llm_memory_graph::types::{Edge, NodeId, TransfersToProperties, Priority};
    ///
    /// let response_id = NodeId::new();
    /// let agent_id = NodeId::new();
    ///
    /// let properties = TransfersToProperties::new(
    ///     "User requested specialist".to_string(),
    ///     "Discussion about quantum computing".to_string(),
    ///     Priority::High,
    /// );
    /// let edge = Edge::transfers_to(response_id, agent_id, properties);
    /// ```
    #[must_use]
    pub fn transfers_to(from: NodeId, to: NodeId, properties: TransfersToProperties) -> Self {
        Self::with_properties(from, to, EdgeType::TransfersTo, properties.to_properties())
    }

    /// Create a REFERENCES edge with typed properties
    ///
    /// Links a prompt to external context it references.
    ///
    /// # Examples
    ///
    /// ```
    /// use llm_memory_graph::types::{Edge, NodeId, ReferencesProperties, ContextType};
    ///
    /// let prompt_id = NodeId::new();
    /// let context_id = NodeId::new();
    ///
    /// let properties = ReferencesProperties::new(
    ///     ContextType::Document,
    ///     0.95,
    ///     Some("chunk_42".to_string()),
    /// );
    /// let edge = Edge::references(prompt_id, context_id, properties);
    /// ```
    #[must_use]
    pub fn references(from: NodeId, to: NodeId, properties: ReferencesProperties) -> Self {
        Self::with_properties(from, to, EdgeType::References, properties.to_properties())
    }

    // ===== Property Extraction Methods =====

    /// Extract INSTANTIATES properties from edge
    ///
    /// Returns None if edge is not of type Instantiates or properties are invalid.
    #[must_use]
    pub fn get_instantiates_properties(&self) -> Option<InstantiatesProperties> {
        if self.edge_type != EdgeType::Instantiates {
            return None;
        }
        InstantiatesProperties::from_properties(&self.properties).ok()
    }

    /// Extract INHERITS properties from edge
    ///
    /// Returns None if edge is not of type Inherits or properties are invalid.
    #[must_use]
    pub fn get_inherits_properties(&self) -> Option<InheritsProperties> {
        if self.edge_type != EdgeType::Inherits {
            return None;
        }
        InheritsProperties::from_properties(&self.properties).ok()
    }

    /// Extract INVOKES properties from edge
    ///
    /// Returns None if edge is not of type Invokes or properties are invalid.
    #[must_use]
    pub fn get_invokes_properties(&self) -> Option<InvokesProperties> {
        if self.edge_type != EdgeType::Invokes {
            return None;
        }
        InvokesProperties::from_properties(&self.properties).ok()
    }

    /// Extract TRANSFERS_TO properties from edge
    ///
    /// Returns None if edge is not of type TransfersTo or properties are invalid.
    #[must_use]
    pub fn get_transfers_to_properties(&self) -> Option<TransfersToProperties> {
        if self.edge_type != EdgeType::TransfersTo {
            return None;
        }
        TransfersToProperties::from_properties(&self.properties).ok()
    }

    /// Extract REFERENCES properties from edge
    ///
    /// Returns None if edge is not of type References or properties are invalid.
    #[must_use]
    pub fn get_references_properties(&self) -> Option<ReferencesProperties> {
        if self.edge_type != EdgeType::References {
            return None;
        }
        ReferencesProperties::from_properties(&self.properties).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_creation() {
        let from = NodeId::new();
        let to = NodeId::new();
        let edge = Edge::new(from, to, EdgeType::Follows);
        assert_eq!(edge.from, from);
        assert_eq!(edge.to, to);
        assert_eq!(edge.edge_type, EdgeType::Follows);
    }

    #[test]
    fn test_edge_properties() {
        let from = NodeId::new();
        let to = NodeId::new();
        let mut edge = Edge::new(from, to, EdgeType::RespondsTo);
        edge.add_property("latency_ms".to_string(), "150".to_string());
        assert_eq!(edge.get_property("latency_ms"), Some(&"150".to_string()));
    }

    #[test]
    fn test_edge_with_properties() {
        let from = NodeId::new();
        let to = NodeId::new();
        let mut props = HashMap::new();
        props.insert("test".to_string(), "value".to_string());
        let edge = Edge::with_properties(from, to, EdgeType::HandledBy, props);
        assert_eq!(edge.properties.len(), 1);
    }

    #[test]
    fn test_invokes_edge_creation() {
        let response_id = NodeId::new();
        let tool_id = NodeId::new();
        let mut edge = Edge::new(response_id, tool_id, EdgeType::Invokes);

        // Add invocation properties
        edge.add_property("invocation_order".to_string(), "1".to_string());
        edge.add_property("success".to_string(), "true".to_string());
        edge.add_property("required".to_string(), "false".to_string());

        assert_eq!(edge.edge_type, EdgeType::Invokes);
        assert_eq!(
            edge.get_property("invocation_order"),
            Some(&"1".to_string())
        );
        assert_eq!(edge.get_property("success"), Some(&"true".to_string()));
    }

    // ===== InstantiatesProperties Tests =====

    #[test]
    fn test_instantiates_properties_creation() {
        let mut bindings = HashMap::new();
        bindings.insert("name".to_string(), "Alice".to_string());
        bindings.insert("age".to_string(), "30".to_string());

        let props = InstantiatesProperties::new("1.2.3".to_string(), bindings.clone());

        assert_eq!(props.template_version, "1.2.3");
        assert_eq!(props.variable_bindings.len(), 2);
        assert_eq!(
            props.variable_bindings.get("name"),
            Some(&"Alice".to_string())
        );
    }

    #[test]
    fn test_instantiates_properties_serialization() {
        let mut bindings = HashMap::new();
        bindings.insert("var1".to_string(), "value1".to_string());

        let props = InstantiatesProperties::new("2.0.0".to_string(), bindings);
        let map = props.to_properties();

        assert_eq!(map.get("template_version"), Some(&"2.0.0".to_string()));
        assert!(map.contains_key("variable_bindings"));
        assert!(map.contains_key("instantiation_time"));
    }

    #[test]
    fn test_instantiates_properties_deserialization() {
        let mut bindings = HashMap::new();
        bindings.insert("key".to_string(), "value".to_string());

        let props = InstantiatesProperties::new("1.0.0".to_string(), bindings);
        let map = props.to_properties();
        let restored = InstantiatesProperties::from_properties(&map).unwrap();

        assert_eq!(restored.template_version, "1.0.0");
        assert_eq!(
            restored.variable_bindings.get("key"),
            Some(&"value".to_string())
        );
    }

    #[test]
    fn test_instantiates_edge_builder() {
        let prompt_id = NodeId::new();
        let template_id = NodeId::new();
        let mut bindings = HashMap::new();
        bindings.insert("name".to_string(), "Bob".to_string());

        let props = InstantiatesProperties::new("3.0.0".to_string(), bindings);
        let edge = Edge::instantiates(prompt_id, template_id, props);

        assert_eq!(edge.edge_type, EdgeType::Instantiates);
        assert_eq!(edge.from, prompt_id);
        assert_eq!(edge.to, template_id);

        let extracted = edge.get_instantiates_properties().unwrap();
        assert_eq!(extracted.template_version, "3.0.0");
    }

    // ===== InheritsProperties Tests =====

    #[test]
    fn test_inherits_properties_creation() {
        let sections = vec!["variables".to_string(), "description".to_string()];
        let props =
            InheritsProperties::new(sections.clone(), "Added validation rules".to_string(), 2);

        assert_eq!(props.override_sections.len(), 2);
        assert_eq!(props.version_diff, "Added validation rules");
        assert_eq!(props.inheritance_depth, 2);
    }

    #[test]
    fn test_inherits_properties_round_trip() {
        let props = InheritsProperties::new(
            vec!["template".to_string()],
            "Changed structure".to_string(),
            1,
        );
        let map = props.to_properties();
        let restored = InheritsProperties::from_properties(&map).unwrap();

        assert_eq!(restored.override_sections.len(), 1);
        assert_eq!(restored.version_diff, "Changed structure");
        assert_eq!(restored.inheritance_depth, 1);
    }

    #[test]
    fn test_inherits_edge_builder() {
        let child_id = NodeId::new();
        let parent_id = NodeId::new();

        let props =
            InheritsProperties::new(vec!["all".to_string()], "Complete rewrite".to_string(), 1);
        let edge = Edge::inherits(child_id, parent_id, props);

        assert_eq!(edge.edge_type, EdgeType::Inherits);
        let extracted = edge.get_inherits_properties().unwrap();
        assert_eq!(extracted.override_sections[0], "all");
    }

    // ===== InvokesProperties Tests =====

    #[test]
    fn test_invokes_properties_creation() {
        let props = InvokesProperties::new(0, true, true);

        assert_eq!(props.invocation_order, 0);
        assert!(props.success);
        assert!(props.required);
    }

    #[test]
    fn test_invokes_properties_serialization() {
        let props = InvokesProperties::new(5, false, true);
        let map = props.to_properties();

        assert_eq!(map.get("invocation_order"), Some(&"5".to_string()));
        assert_eq!(map.get("success"), Some(&"false".to_string()));
        assert_eq!(map.get("required"), Some(&"true".to_string()));
    }

    #[test]
    fn test_invokes_properties_deserialization() {
        let props = InvokesProperties::new(3, true, false);
        let map = props.to_properties();
        let restored = InvokesProperties::from_properties(&map).unwrap();

        assert_eq!(restored.invocation_order, 3);
        assert!(restored.success);
        assert!(!restored.required);
    }

    #[test]
    fn test_invokes_edge_builder() {
        let response_id = NodeId::new();
        let tool_id = NodeId::new();

        let props = InvokesProperties::new(1, true, true);
        let edge = Edge::invokes(response_id, tool_id, props);

        assert_eq!(edge.edge_type, EdgeType::Invokes);
        let extracted = edge.get_invokes_properties().unwrap();
        assert_eq!(extracted.invocation_order, 1);
        assert!(extracted.success);
    }

    // ===== Priority Tests =====

    #[test]
    fn test_priority_display() {
        assert_eq!(Priority::Low.to_string(), "low");
        assert_eq!(Priority::Normal.to_string(), "normal");
        assert_eq!(Priority::High.to_string(), "high");
        assert_eq!(Priority::Critical.to_string(), "critical");
    }

    #[test]
    fn test_priority_from_str() {
        assert_eq!("low".parse::<Priority>().unwrap(), Priority::Low);
        assert_eq!("normal".parse::<Priority>().unwrap(), Priority::Normal);
        assert_eq!("high".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("critical".parse::<Priority>().unwrap(), Priority::Critical);
        assert_eq!("NORMAL".parse::<Priority>().unwrap(), Priority::Normal);
        assert!("invalid".parse::<Priority>().is_err());
    }

    // ===== TransfersToProperties Tests =====

    #[test]
    fn test_transfers_to_properties_creation() {
        let props = TransfersToProperties::new(
            "User needs expert help".to_string(),
            "Discussed technical architecture".to_string(),
            Priority::High,
        );

        assert_eq!(props.handoff_reason, "User needs expert help");
        assert_eq!(props.context_summary, "Discussed technical architecture");
        assert_eq!(props.priority, Priority::High);
    }

    #[test]
    fn test_transfers_to_properties_round_trip() {
        let props = TransfersToProperties::new(
            "Escalation needed".to_string(),
            "Complex issue".to_string(),
            Priority::Critical,
        );
        let map = props.to_properties();
        let restored = TransfersToProperties::from_properties(&map).unwrap();

        assert_eq!(restored.handoff_reason, "Escalation needed");
        assert_eq!(restored.priority, Priority::Critical);
    }

    #[test]
    fn test_transfers_to_edge_builder() {
        let response_id = NodeId::new();
        let agent_id = NodeId::new();

        let props = TransfersToProperties::new(
            "Specialist required".to_string(),
            "Technical query".to_string(),
            Priority::Normal,
        );
        let edge = Edge::transfers_to(response_id, agent_id, props);

        assert_eq!(edge.edge_type, EdgeType::TransfersTo);
        let extracted = edge.get_transfers_to_properties().unwrap();
        assert_eq!(extracted.handoff_reason, "Specialist required");
    }

    // ===== ContextType Tests =====

    #[test]
    fn test_context_type_display() {
        assert_eq!(ContextType::Document.to_string(), "document");
        assert_eq!(ContextType::WebPage.to_string(), "webpage");
        assert_eq!(ContextType::Database.to_string(), "database");
        assert_eq!(ContextType::VectorSearch.to_string(), "vector_search");
        assert_eq!(ContextType::Memory.to_string(), "memory");
    }

    #[test]
    fn test_context_type_from_str() {
        assert_eq!(
            "document".parse::<ContextType>().unwrap(),
            ContextType::Document
        );
        assert_eq!(
            "webpage".parse::<ContextType>().unwrap(),
            ContextType::WebPage
        );
        assert_eq!(
            "database".parse::<ContextType>().unwrap(),
            ContextType::Database
        );
        assert_eq!(
            "vector_search".parse::<ContextType>().unwrap(),
            ContextType::VectorSearch
        );
        assert_eq!(
            "memory".parse::<ContextType>().unwrap(),
            ContextType::Memory
        );
        assert_eq!(
            "DOCUMENT".parse::<ContextType>().unwrap(),
            ContextType::Document
        );
        assert!("invalid".parse::<ContextType>().is_err());
    }

    // ===== ReferencesProperties Tests =====

    #[test]
    fn test_references_properties_creation() {
        let props =
            ReferencesProperties::new(ContextType::Document, 0.95, Some("chunk_42".to_string()));

        assert_eq!(props.context_type, ContextType::Document);
        assert_eq!(props.relevance_score, 0.95);
        assert_eq!(props.chunk_id, Some("chunk_42".to_string()));
    }

    #[test]
    fn test_references_properties_relevance_clamping() {
        // Test clamping above 1.0
        let props1 = ReferencesProperties::new(ContextType::WebPage, 1.5, None);
        assert_eq!(props1.relevance_score, 1.0);

        // Test clamping below 0.0
        let props2 = ReferencesProperties::new(ContextType::Database, -0.5, None);
        assert_eq!(props2.relevance_score, 0.0);

        // Test valid range
        let props3 = ReferencesProperties::new(ContextType::Memory, 0.75, None);
        assert_eq!(props3.relevance_score, 0.75);
    }

    #[test]
    fn test_references_properties_round_trip() {
        let props =
            ReferencesProperties::new(ContextType::VectorSearch, 0.88, Some("doc_123".to_string()));
        let map = props.to_properties();
        let restored = ReferencesProperties::from_properties(&map).unwrap();

        assert_eq!(restored.context_type, ContextType::VectorSearch);
        assert!((restored.relevance_score - 0.88).abs() < 0.01);
        assert_eq!(restored.chunk_id, Some("doc_123".to_string()));
    }

    #[test]
    fn test_references_edge_builder() {
        let prompt_id = NodeId::new();
        let context_id = NodeId::new();

        let props =
            ReferencesProperties::new(ContextType::Document, 0.92, Some("section_5".to_string()));
        let edge = Edge::references(prompt_id, context_id, props);

        assert_eq!(edge.edge_type, EdgeType::References);
        let extracted = edge.get_references_properties().unwrap();
        assert_eq!(extracted.context_type, ContextType::Document);
        assert_eq!(extracted.chunk_id, Some("section_5".to_string()));
    }

    // ===== Property Extraction Tests =====

    #[test]
    fn test_property_extraction_wrong_type() {
        let from = NodeId::new();
        let to = NodeId::new();
        let edge = Edge::new(from, to, EdgeType::Follows);

        // Should return None for wrong edge type
        assert!(edge.get_instantiates_properties().is_none());
        assert!(edge.get_inherits_properties().is_none());
        assert!(edge.get_invokes_properties().is_none());
        assert!(edge.get_transfers_to_properties().is_none());
        assert!(edge.get_references_properties().is_none());
    }

    #[test]
    fn test_all_edge_types_compile() {
        let from = NodeId::new();
        let to = NodeId::new();

        // Test all edge types can be created
        let _follows = Edge::new(from, to, EdgeType::Follows);
        let _responds_to = Edge::new(from, to, EdgeType::RespondsTo);
        let _handled_by = Edge::new(from, to, EdgeType::HandledBy);
        let _part_of = Edge::new(from, to, EdgeType::PartOf);
        let _invokes = Edge::new(from, to, EdgeType::Invokes);
        let _transfers_to = Edge::new(from, to, EdgeType::TransfersTo);
        let _instantiates = Edge::new(from, to, EdgeType::Instantiates);
        let _inherits = Edge::new(from, to, EdgeType::Inherits);
        let _references = Edge::new(from, to, EdgeType::References);
    }
}
