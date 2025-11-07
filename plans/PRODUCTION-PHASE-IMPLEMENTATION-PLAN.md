# LLM-Memory-Graph: Production Phase Implementation Plan

**Version**: 1.0 Production
**Status**: Planning
**Target Timeline**: 8 weeks
**Prerequisites**: Beta Phase Complete ✅
**Document Date**: 2025-11-07

---

## Executive Summary

This plan outlines the Production Phase implementation for LLM-Memory-Graph, transforming the Beta system into an enterprise-grade, production-ready distributed service with plugin architecture, ecosystem integration, and comprehensive observability.

### Production Phase Objectives

1. **gRPC Standalone Service**: High-performance distributed service with streaming support
2. **Plugin System**: Extensible architecture for custom functionality
3. **LLM-Registry Integration**: Metadata and version tracking
4. **Data-Vault Integration**: Secure archival and compliance
5. **Enhanced Prometheus Metrics**: Production-grade observability (extends existing implementation)

### Key Deliverables

- ✅ gRPC service with full CRUD API
- ✅ Plugin SDK and reference implementations
- ✅ LLM-Registry client integration
- ✅ Data-Vault archival pipeline
- ✅ Production-grade Prometheus metrics
- ✅ Docker/Kubernetes deployment manifests
- ✅ Comprehensive documentation and runbooks

---

## Table of Contents

1. [Technical Architecture](#1-technical-architecture)
2. [gRPC Service Implementation](#2-grpc-service-implementation)
3. [Plugin System Architecture](#3-plugin-system-architecture)
4. [LLM-Registry Integration](#4-llm-registry-integration)
5. [Data-Vault Integration](#5-data-vault-integration)
6. [Enhanced Prometheus Metrics](#6-enhanced-prometheus-metrics)
7. [Deployment Architecture](#7-deployment-architecture)
8. [Implementation Roadmap](#8-implementation-roadmap)
9. [Testing Strategy](#9-testing-strategy)
10. [Security & Compliance](#10-security--compliance)
11. [Performance Targets](#11-performance-targets)
12. [Success Metrics](#12-success-metrics)

---

## 1. Technical Architecture

### 1.1 System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Production Architecture                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐      ┌──────────────────┐      ┌──────────────────┐      │
│  │   gRPC       │      │  LLM-Memory-     │      │   LLM-           │      │
│  │   Server     │─────▶│  Graph Engine    │─────▶│   Observatory    │      │
│  │ (Tonic)      │      │  (w/ Plugins)    │      │   (Events)       │      │
│  └──────┬───────┘      └────────┬─────────┘      └──────────────────┘      │
│         │                       │                                           │
│         │                       ▼                                           │
│         │            ┌──────────────────┐      ┌──────────────────┐        │
│         │            │  Storage Backend │      │   LLM-Registry   │        │
│         │            │  (Async Sled)    │─────▶│   (Metadata)     │        │
│         │            └──────────────────┘      └──────────────────┘        │
│         │                       │                                           │
│         │                       ▼                                           │
│         │            ┌──────────────────┐      ┌──────────────────┐        │
│         └───────────▶│   Prometheus     │      │   Data-Vault     │        │
│                      │   Metrics        │      │   (Archive)      │        │
│                      └──────────────────┘      └──────────────────┘        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Module Structure (Production)

```
llm-memory-graph/
├── src/
│   ├── lib.rs                         # Core library exports
│   ├── bin/
│   │   └── server.rs                  # [NEW] gRPC server binary
│   ├── grpc/                          # [NEW] gRPC implementation
│   │   ├── mod.rs                     # Public API
│   │   ├── service.rs                 # Service implementation
│   │   ├── handlers.rs                # Request handlers
│   │   └── streaming.rs               # Stream handlers
│   ├── plugin/                        # [NEW] Plugin system
│   │   ├── mod.rs                     # Plugin API
│   │   ├── manager.rs                 # Plugin lifecycle
│   │   ├── registry.rs                # Plugin registry
│   │   └── hooks.rs                   # Plugin hooks
│   ├── integrations/                  # [NEW] External integrations
│   │   ├── mod.rs                     # Integration exports
│   │   ├── registry/                  # LLM-Registry client
│   │   │   ├── mod.rs
│   │   │   ├── client.rs
│   │   │   └── types.rs
│   │   └── vault/                     # Data-Vault client
│   │       ├── mod.rs
│   │       ├── archiver.rs
│   │       └── retention.rs
│   ├── observatory/                   # [ENHANCED] Observatory
│   │   ├── prometheus.rs              # Enhanced Prometheus metrics
│   │   └── ...                        # Existing modules
│   └── ...                            # Existing modules
├── proto/                             # [NEW] Protocol definitions
│   └── memory_graph.proto
├── plugins/                           # [NEW] Plugin examples
│   ├── example_validator/
│   └── example_enricher/
└── deploy/                            # [NEW] Deployment configs
    ├── docker/
    ├── kubernetes/
    └── helm/
```

---

## 2. gRPC Service Implementation

### 2.1 Service Design

#### 2.1.1 Protocol Definition (`proto/memory_graph.proto`)

```protobuf
syntax = "proto3";

package llm.memory.graph.v1;

import "google/protobuf/timestamp.proto";
import "google/protobuf/empty.proto";

// ============================================================================
// SERVICE DEFINITION
// ============================================================================

service MemoryGraphService {
  // Session Management
  rpc CreateSession(CreateSessionRequest) returns (Session);
  rpc GetSession(GetSessionRequest) returns (Session);
  rpc DeleteSession(DeleteSessionRequest) returns (google.protobuf.Empty);
  rpc ListSessions(ListSessionsRequest) returns (ListSessionsResponse);

  // Node Operations
  rpc CreateNode(CreateNodeRequest) returns (Node);
  rpc GetNode(GetNodeRequest) returns (Node);
  rpc UpdateNode(UpdateNodeRequest) returns (Node);
  rpc DeleteNode(DeleteNodeRequest) returns (google.protobuf.Empty);
  rpc BatchCreateNodes(BatchCreateNodesRequest) returns (BatchCreateNodesResponse);
  rpc BatchGetNodes(BatchGetNodesRequest) returns (BatchGetNodesResponse);

  // Edge Operations
  rpc CreateEdge(CreateEdgeRequest) returns (Edge);
  rpc GetEdges(GetEdgesRequest) returns (GetEdgesResponse);
  rpc DeleteEdge(DeleteEdgeRequest) returns (google.protobuf.Empty);

  // Query Operations
  rpc Query(QueryRequest) returns (QueryResponse);
  rpc StreamQuery(QueryRequest) returns (stream Node);

  // Prompt & Response Operations
  rpc AddPrompt(AddPromptRequest) returns (PromptNode);
  rpc AddResponse(AddResponseRequest) returns (ResponseNode);
  rpc AddToolInvocation(AddToolInvocationRequest) returns (ToolInvocationNode);

  // Template Operations
  rpc CreateTemplate(CreateTemplateRequest) returns (TemplateNode);
  rpc InstantiateTemplate(InstantiateTemplateRequest) returns (PromptNode);

  // Streaming Operations
  rpc StreamEvents(StreamEventsRequest) returns (stream Event);
  rpc SubscribeToSession(SubscribeRequest) returns (stream SessionEvent);

  // Health & Metrics
  rpc Health(google.protobuf.Empty) returns (HealthResponse);
  rpc GetMetrics(google.protobuf.Empty) returns (MetricsResponse);
}

// ============================================================================
// DATA TYPES
// ============================================================================

message Session {
  string id = 1;
  google.protobuf.Timestamp created_at = 2;
  google.protobuf.Timestamp updated_at = 3;
  map<string, string> metadata = 4;
  bool is_active = 5;
}

message Node {
  string id = 1;
  NodeType type = 2;
  google.protobuf.Timestamp created_at = 3;
  oneof node_data {
    PromptNode prompt = 10;
    ResponseNode response = 11;
    ToolInvocationNode tool_invocation = 12;
    AgentNode agent = 13;
    TemplateNode template = 14;
  }
}

enum NodeType {
  NODE_TYPE_UNSPECIFIED = 0;
  NODE_TYPE_SESSION = 1;
  NODE_TYPE_PROMPT = 2;
  NODE_TYPE_RESPONSE = 3;
  NODE_TYPE_TOOL_INVOCATION = 4;
  NODE_TYPE_AGENT = 5;
  NODE_TYPE_TEMPLATE = 6;
}

message PromptNode {
  string id = 1;
  string session_id = 2;
  string content = 3;
  google.protobuf.Timestamp timestamp = 4;
  PromptMetadata metadata = 5;
}

message ResponseNode {
  string id = 1;
  string prompt_id = 2;
  string content = 3;
  google.protobuf.Timestamp timestamp = 4;
  TokenUsage token_usage = 5;
  ResponseMetadata metadata = 6;
}

message ToolInvocationNode {
  string id = 1;
  string response_id = 2;
  string tool_name = 3;
  string parameters = 4;  // JSON
  string status = 5;
  string result = 6;  // JSON
  string error = 7;
  int64 duration_ms = 8;
  int32 retry_count = 9;
  google.protobuf.Timestamp timestamp = 10;
  map<string, string> metadata = 11;
}

message AgentNode {
  string id = 1;
  string name = 2;
  string role = 3;
  repeated string capabilities = 4;
  string status = 5;
  google.protobuf.Timestamp created_at = 6;
  map<string, string> metadata = 7;
}

message TemplateNode {
  string id = 1;
  string name = 2;
  string template_text = 3;
  repeated VariableSpec variables = 4;
  string version = 5;
  int64 usage_count = 6;
  google.protobuf.Timestamp created_at = 7;
  map<string, string> metadata = 8;
}

message Edge {
  string id = 1;
  string from_node_id = 2;
  string to_node_id = 3;
  EdgeType type = 4;
  google.protobuf.Timestamp created_at = 5;
  map<string, string> properties = 6;
}

enum EdgeType {
  EDGE_TYPE_UNSPECIFIED = 0;
  EDGE_TYPE_BELONGS_TO = 1;
  EDGE_TYPE_RESPONDS_TO = 2;
  EDGE_TYPE_FOLLOWS = 3;
  EDGE_TYPE_INVOKES = 4;
  EDGE_TYPE_HANDLED_BY = 5;
  EDGE_TYPE_INSTANTIATES = 6;
  EDGE_TYPE_INHERITS = 7;
  EDGE_TYPE_TRANSFERS_TO = 8;
  EDGE_TYPE_REFERENCES = 9;
}

message TokenUsage {
  int64 prompt_tokens = 1;
  int64 completion_tokens = 2;
  int64 total_tokens = 3;
}

message PromptMetadata {
  string model = 1;
  double temperature = 2;
  optional int32 max_tokens = 3;
  repeated string tools_available = 4;
  map<string, string> custom = 5;
}

message ResponseMetadata {
  string model = 1;
  string finish_reason = 2;
  int64 latency_ms = 3;
  map<string, string> custom = 4;
}

message VariableSpec {
  string name = 1;
  string type_hint = 2;
  bool required = 3;
  optional string default_value = 4;
  optional string validation_pattern = 5;
  string description = 6;
}

// ============================================================================
// REQUEST/RESPONSE MESSAGES
// ============================================================================

message CreateSessionRequest {
  map<string, string> metadata = 1;
}

message GetSessionRequest {
  string session_id = 1;
}

message DeleteSessionRequest {
  string session_id = 1;
}

message ListSessionsRequest {
  int32 limit = 1;
  int32 offset = 2;
}

message ListSessionsResponse {
  repeated Session sessions = 1;
  int64 total_count = 2;
}

message CreateNodeRequest {
  Node node = 1;
}

message GetNodeRequest {
  string node_id = 1;
}

message UpdateNodeRequest {
  Node node = 1;
}

message DeleteNodeRequest {
  string node_id = 1;
}

message BatchCreateNodesRequest {
  repeated Node nodes = 1;
}

message BatchCreateNodesResponse {
  repeated Node nodes = 1;
  int32 created_count = 2;
}

message BatchGetNodesRequest {
  repeated string node_ids = 1;
}

message BatchGetNodesResponse {
  repeated Node nodes = 1;
}

message CreateEdgeRequest {
  Edge edge = 1;
}

message GetEdgesRequest {
  string node_id = 1;
  optional EdgeDirection direction = 2;
  optional EdgeType type = 3;
}

enum EdgeDirection {
  EDGE_DIRECTION_UNSPECIFIED = 0;
  EDGE_DIRECTION_OUTGOING = 1;
  EDGE_DIRECTION_INCOMING = 2;
  EDGE_DIRECTION_BOTH = 3;
}

message GetEdgesResponse {
  repeated Edge edges = 1;
}

message DeleteEdgeRequest {
  string edge_id = 1;
}

message QueryRequest {
  optional string session_id = 1;
  optional NodeType node_type = 2;
  optional google.protobuf.Timestamp after = 3;
  optional google.protobuf.Timestamp before = 4;
  int32 limit = 5;
  int32 offset = 6;
  map<string, string> filters = 7;
}

message QueryResponse {
  repeated Node nodes = 1;
  int64 total_count = 2;
}

message AddPromptRequest {
  string session_id = 1;
  string content = 2;
  optional PromptMetadata metadata = 3;
}

message AddResponseRequest {
  string prompt_id = 1;
  string content = 2;
  TokenUsage token_usage = 3;
  optional ResponseMetadata metadata = 4;
}

message AddToolInvocationRequest {
  ToolInvocationNode tool_invocation = 1;
}

message CreateTemplateRequest {
  TemplateNode template = 1;
}

message InstantiateTemplateRequest {
  string template_id = 1;
  map<string, string> variable_values = 2;
  string session_id = 3;
}

message StreamEventsRequest {
  optional string session_id = 1;
  repeated EventType event_types = 2;
}

message Event {
  string id = 1;
  EventType type = 2;
  google.protobuf.Timestamp timestamp = 3;
  string payload = 4;  // JSON
}

enum EventType {
  EVENT_TYPE_UNSPECIFIED = 0;
  EVENT_TYPE_NODE_CREATED = 1;
  EVENT_TYPE_NODE_UPDATED = 2;
  EVENT_TYPE_NODE_DELETED = 3;
  EVENT_TYPE_EDGE_CREATED = 4;
  EVENT_TYPE_EDGE_DELETED = 5;
  EVENT_TYPE_SESSION_CREATED = 6;
  EVENT_TYPE_SESSION_CLOSED = 7;
}

message SubscribeRequest {
  string session_id = 1;
}

message SessionEvent {
  Event event = 1;
  string session_id = 2;
}

message HealthResponse {
  enum ServingStatus {
    UNKNOWN = 0;
    SERVING = 1;
    NOT_SERVING = 2;
  }
  ServingStatus status = 1;
  string version = 2;
  int64 uptime_seconds = 3;
}

message MetricsResponse {
  int64 total_nodes = 1;
  int64 total_edges = 2;
  int64 total_sessions = 3;
  int64 active_sessions = 4;
  double avg_write_latency_ms = 5;
  double avg_read_latency_ms = 6;
  int64 requests_per_second = 7;
}
```

#### 2.1.2 Service Implementation (`src/grpc/service.rs`)

```rust
//! gRPC service implementation for LLM-Memory-Graph

use crate::engine::AsyncMemoryGraph;
use crate::grpc::proto::{
    memory_graph_service_server::MemoryGraphService,
    *
};
use crate::observatory::PrometheusMetrics;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status, Code};
use tracing::{info, warn, error, instrument};

/// gRPC service implementation
pub struct MemoryGraphServiceImpl {
    graph: Arc<AsyncMemoryGraph>,
    metrics: Arc<PrometheusMetrics>,
    plugin_manager: Arc<RwLock<PluginManager>>,
    config: ServiceConfig,
}

impl MemoryGraphServiceImpl {
    pub fn new(
        graph: Arc<AsyncMemoryGraph>,
        metrics: Arc<PrometheusMetrics>,
        plugin_manager: Arc<RwLock<PluginManager>>,
        config: ServiceConfig,
    ) -> Self {
        Self {
            graph,
            metrics,
            plugin_manager,
            config,
        }
    }

    /// Convert internal error to gRPC status
    fn map_error(err: crate::Error) -> Status {
        match err {
            crate::Error::NotFound(_) => Status::not_found(err.to_string()),
            crate::Error::InvalidInput(_) => Status::invalid_argument(err.to_string()),
            crate::Error::Timeout(_) => Status::deadline_exceeded(err.to_string()),
            crate::Error::Storage(_) => Status::internal(err.to_string()),
            _ => Status::internal(err.to_string()),
        }
    }

    /// Execute plugin hooks before operation
    async fn execute_before_hooks(
        &self,
        hook_type: &str,
        context: &PluginContext,
    ) -> Result<(), Status> {
        let plugins = self.plugin_manager.read().await;
        for plugin in plugins.active_plugins() {
            plugin.before_hook(hook_type, context)
                .await
                .map_err(|e| Status::internal(format!("Plugin hook failed: {}", e)))?;
        }
        Ok(())
    }

    /// Execute plugin hooks after operation
    async fn execute_after_hooks(
        &self,
        hook_type: &str,
        context: &PluginContext,
    ) -> Result<(), Status> {
        let plugins = self.plugin_manager.read().await;
        for plugin in plugins.active_plugins() {
            plugin.after_hook(hook_type, context)
                .await
                .map_err(|e| Status::internal(format!("Plugin hook failed: {}", e)))?;
        }
        Ok(())
    }
}

#[tonic::async_trait]
impl MemoryGraphService for MemoryGraphServiceImpl {
    #[instrument(skip(self))]
    async fn create_session(
        &self,
        request: Request<CreateSessionRequest>,
    ) -> Result<Response<Session>, Status> {
        let req = request.into_inner();

        // Plugin hook: before_create_session
        let context = PluginContext::new("create_session", serde_json::json!(&req));
        self.execute_before_hooks("before_create_session", &context).await?;

        // Create session
        let session = self.graph.create_session_with_metadata(req.metadata)
            .await
            .map_err(Self::map_error)?;

        // Convert to proto
        let proto_session = Session {
            id: session.id.to_string(),
            created_at: Some(prost_types::Timestamp {
                seconds: session.created_at.timestamp(),
                nanos: session.created_at.timestamp_subsec_nanos() as i32,
            }),
            updated_at: Some(prost_types::Timestamp {
                seconds: session.updated_at.timestamp(),
                nanos: session.updated_at.timestamp_subsec_nanos() as i32,
            }),
            metadata: session.metadata,
            is_active: session.is_active,
        };

        // Plugin hook: after_create_session
        self.execute_after_hooks("after_create_session", &context).await?;

        // Update metrics
        self.metrics.active_sessions.inc();

        info!("Created session: {}", session.id);
        Ok(Response::new(proto_session))
    }

    #[instrument(skip(self))]
    async fn get_session(
        &self,
        request: Request<GetSessionRequest>,
    ) -> Result<Response<Session>, Status> {
        let req = request.into_inner();
        let session_id = SessionId::parse_str(&req.session_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid session ID: {}", e)))?;

        let session = self.graph.get_session(session_id)
            .await
            .map_err(Self::map_error)?;

        let proto_session = Session {
            id: session.id.to_string(),
            created_at: Some(prost_types::Timestamp {
                seconds: session.created_at.timestamp(),
                nanos: session.created_at.timestamp_subsec_nanos() as i32,
            }),
            updated_at: Some(prost_types::Timestamp {
                seconds: session.updated_at.timestamp(),
                nanos: session.updated_at.timestamp_subsec_nanos() as i32,
            }),
            metadata: session.metadata,
            is_active: session.is_active,
        };

        Ok(Response::new(proto_session))
    }

    #[instrument(skip(self))]
    async fn add_prompt(
        &self,
        request: Request<AddPromptRequest>,
    ) -> Result<Response<PromptNode>, Status> {
        let start = std::time::Instant::now();
        let req = request.into_inner();

        let session_id = SessionId::parse_str(&req.session_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid session ID: {}", e)))?;

        // Convert metadata
        let metadata = req.metadata.map(|m| crate::PromptMetadata {
            model: m.model,
            temperature: m.temperature,
            max_tokens: m.max_tokens.map(|t| t as usize),
            tools_available: m.tools_available,
            custom: m.custom,
        });

        let prompt_id = self.graph.add_prompt(session_id, req.content, metadata)
            .await
            .map_err(Self::map_error)?;

        let node = self.graph.get_node(&prompt_id)
            .await
            .map_err(Self::map_error)?;

        let proto_node = match node {
            crate::types::Node::Prompt(p) => PromptNode {
                id: p.id.to_string(),
                session_id: p.session_id.to_string(),
                content: p.content,
                timestamp: Some(prost_types::Timestamp {
                    seconds: p.timestamp.timestamp(),
                    nanos: p.timestamp.timestamp_subsec_nanos() as i32,
                }),
                metadata: p.metadata.map(|m| proto::PromptMetadata {
                    model: m.model,
                    temperature: m.temperature,
                    max_tokens: m.max_tokens.map(|t| t as i32),
                    tools_available: m.tools_available,
                    custom: m.custom,
                }),
            },
            _ => return Err(Status::internal("Unexpected node type")),
        };

        // Update metrics
        self.metrics.prompts_submitted.inc();
        let duration = start.elapsed();
        self.metrics.write_latency.observe(duration.as_secs_f64());

        Ok(Response::new(proto_node))
    }

    #[instrument(skip(self))]
    async fn stream_query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<Self::StreamQueryStream>, Status> {
        let req = request.into_inner();

        // Build query
        let mut query = self.graph.query();

        if let Some(session_id) = req.session_id {
            let sid = SessionId::parse_str(&session_id)
                .map_err(|e| Status::invalid_argument(format!("Invalid session ID: {}", e)))?;
            query = query.session(sid);
        }

        if let Some(node_type) = req.node_type {
            query = query.node_type(convert_node_type(node_type)?);
        }

        if req.limit > 0 {
            query = query.limit(req.limit as usize);
        }

        if req.offset > 0 {
            query = query.offset(req.offset as usize);
        }

        // Execute streaming query
        let stream = query.execute_stream();

        // Convert to gRPC stream
        let output_stream = stream.map(|result| {
            result
                .map(|node| convert_to_proto_node(node))
                .map_err(Self::map_error)
        });

        Ok(Response::new(Box::pin(output_stream)))
    }

    #[instrument(skip(self))]
    async fn health(
        &self,
        _request: Request<()>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            status: health_response::ServingStatus::Serving as i32,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: self.config.start_time.elapsed().as_secs() as i64,
        }))
    }

    #[instrument(skip(self))]
    async fn get_metrics(
        &self,
        _request: Request<()>,
    ) -> Result<Response<MetricsResponse>, Status> {
        let metrics = self.graph.get_metrics()
            .ok_or_else(|| Status::unavailable("Metrics not available"))?;

        Ok(Response::new(MetricsResponse {
            total_nodes: metrics.nodes_created as i64,
            total_edges: metrics.edges_created as i64,
            total_sessions: metrics.sessions_created as i64,
            active_sessions: self.metrics.active_sessions.get() as i64,
            avg_write_latency_ms: metrics.avg_write_latency_ms,
            avg_read_latency_ms: metrics.avg_read_latency_ms,
            requests_per_second: 0, // TODO: Calculate from metrics
        }))
    }

    // TODO: Implement remaining methods...
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub request_timeout_ms: u64,
    pub enable_reflection: bool,
    pub enable_health: bool,
    pub start_time: std::time::Instant,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 50051,
            max_connections: 1000,
            request_timeout_ms: 30000,
            enable_reflection: true,
            enable_health: true,
            start_time: std::time::Instant::now(),
        }
    }
}
```

### 2.2 Server Binary (`src/bin/server.rs`)

```rust
//! gRPC server binary for LLM-Memory-Graph

use llm_memory_graph::{
    engine::AsyncMemoryGraph,
    grpc::service::{MemoryGraphServiceImpl, ServiceConfig},
    observatory::PrometheusMetrics,
    plugin::PluginManager,
    types::Config,
};
use prometheus::Registry;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting LLM-Memory-Graph gRPC Server v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "./data".to_string());
    let host = std::env::var("GRPC_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("GRPC_PORT")
        .unwrap_or_else(|_| "50051".to_string())
        .parse()?;

    // Initialize Prometheus registry
    let registry = Registry::new();
    let metrics = Arc::new(PrometheusMetrics::new(&registry)?);

    // Initialize memory graph
    let config = Config::new(&db_path);
    let graph = Arc::new(AsyncMemoryGraph::open(config).await?);

    // Initialize plugin manager
    let plugin_manager = Arc::new(RwLock::new(PluginManager::new()));

    // Load plugins from environment
    if let Ok(plugin_dirs) = std::env::var("PLUGIN_DIRS") {
        let mut manager = plugin_manager.write().await;
        for dir in plugin_dirs.split(',') {
            manager.load_from_directory(dir).await?;
        }
    }

    // Create service
    let service_config = ServiceConfig {
        host: host.clone(),
        port,
        ..Default::default()
    };

    let service = MemoryGraphServiceImpl::new(
        graph,
        metrics.clone(),
        plugin_manager,
        service_config,
    );

    // Build server
    let addr = format!("{}:{}", host, port).parse()?;
    info!("gRPC server listening on {}", addr);

    let server = Server::builder()
        .add_service(memory_graph_service_server::MemoryGraphServiceServer::new(service))
        .serve(addr);

    // Spawn metrics server
    tokio::spawn(async move {
        serve_metrics(registry, metrics).await;
    });

    // Start server
    server.await?;

    Ok(())
}

/// Serve Prometheus metrics on separate HTTP port
async fn serve_metrics(registry: Registry, _metrics: Arc<PrometheusMetrics>) {
    use warp::Filter;

    let metrics_route = warp::path("metrics").map(move || {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = registry.gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    });

    let port: u16 = std::env::var("METRICS_PORT")
        .unwrap_or_else(|_| "9090".to_string())
        .parse()
        .unwrap();

    info!("Metrics server listening on http://0.0.0.0:{}/metrics", port);
    warp::serve(metrics_route).run(([0, 0, 0, 0], port)).await;
}
```

### 2.3 Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies

# gRPC
tonic = "0.11"
prost = "0.12"
prost-types = "0.12"

# HTTP server for metrics
warp = "0.3"
hyper = "0.14"

[build-dependencies]
tonic-build = "0.11"

[[bin]]
name = "server"
path = "src/bin/server.rs"
```

Create `build.rs`:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/grpc/proto")
        .compile(&["proto/memory_graph.proto"], &["proto"])?;
    Ok(())
}
```

---

## 3. Plugin System Architecture

### 3.1 Plugin API Design

#### 3.1.1 Plugin Trait (`src/plugin/mod.rs`)

```rust
//! Plugin system for extending LLM-Memory-Graph functionality

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error as StdError;

/// Plugin error type
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin initialization failed: {0}")]
    InitFailed(String),

    #[error("Plugin hook execution failed: {0}")]
    HookFailed(String),

    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin version incompatible: {0}")]
    VersionMismatch(String),
}

/// Plugin metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub api_version: String,
    pub capabilities: Vec<String>,
}

/// Plugin context for hooks
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub operation: String,
    pub data: Value,
    pub metadata: HashMap<String, String>,
}

impl PluginContext {
    pub fn new(operation: impl Into<String>, data: Value) -> Self {
        Self {
            operation: operation.into(),
            data,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Plugin trait - all plugins must implement this
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize plugin
    async fn init(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    /// Shutdown plugin
    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: Before node creation
    async fn before_create_node(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: After node creation
    async fn after_create_node(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: Before session creation
    async fn before_create_session(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: After session creation
    async fn after_create_session(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: Before query execution
    async fn before_query(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: After query execution
    async fn after_query(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Generic hook execution
    async fn before_hook(&self, hook_name: &str, context: &PluginContext) -> Result<(), PluginError> {
        match hook_name {
            "before_create_node" => self.before_create_node(context).await,
            "before_create_session" => self.before_create_session(context).await,
            "before_query" => self.before_query(context).await,
            _ => Ok(()),
        }
    }

    /// Generic hook execution
    async fn after_hook(&self, hook_name: &str, context: &PluginContext) -> Result<(), PluginError> {
        match hook_name {
            "after_create_node" => self.after_create_node(context).await,
            "after_create_session" => self.after_create_session(context).await,
            "after_query" => self.after_query(context).await,
            _ => Ok(()),
        }
    }
}

/// Plugin builder for configuration
pub struct PluginBuilder {
    metadata: PluginMetadata,
}

impl PluginBuilder {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            metadata: PluginMetadata {
                name: name.into(),
                version: version.into(),
                author: String::new(),
                description: String::new(),
                api_version: "1.0.0".to_string(),
                capabilities: Vec::new(),
            },
        }
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.metadata.author = author.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.metadata.description = description.into();
        self
    }

    pub fn capability(mut self, capability: impl Into<String>) -> Self {
        self.metadata.capabilities.push(capability.into());
        self
    }

    pub fn build(self) -> PluginMetadata {
        self.metadata
    }
}
```

#### 3.1.2 Plugin Manager (`src/plugin/manager.rs`)

```rust
//! Plugin lifecycle management

use super::{Plugin, PluginError, PluginMetadata};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn, error};

/// Plugin manager
pub struct PluginManager {
    plugins: HashMap<String, Arc<dyn Plugin>>,
    enabled: HashMap<String, bool>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            enabled: HashMap::new(),
        }
    }

    /// Register a plugin
    pub async fn register(&mut self, plugin: Arc<dyn Plugin>) -> Result<(), PluginError> {
        let metadata = plugin.metadata();
        let name = metadata.name.clone();

        info!("Registering plugin: {} v{}", name, metadata.version);

        // Check API version compatibility
        if metadata.api_version != "1.0.0" {
            return Err(PluginError::VersionMismatch(format!(
                "Plugin {} requires API version {}, but 1.0.0 is supported",
                name, metadata.api_version
            )));
        }

        self.plugins.insert(name.clone(), plugin);
        self.enabled.insert(name, true);

        Ok(())
    }

    /// Enable a plugin
    pub fn enable(&mut self, name: &str) -> Result<(), PluginError> {
        if !self.plugins.contains_key(name) {
            return Err(PluginError::NotFound(name.to_string()));
        }
        self.enabled.insert(name.to_string(), true);
        info!("Enabled plugin: {}", name);
        Ok(())
    }

    /// Disable a plugin
    pub fn disable(&mut self, name: &str) -> Result<(), PluginError> {
        if !self.plugins.contains_key(name) {
            return Err(PluginError::NotFound(name.to_string()));
        }
        self.enabled.insert(name.to_string(), false);
        info!("Disabled plugin: {}", name);
        Ok(())
    }

    /// Get active plugins
    pub fn active_plugins(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins
            .iter()
            .filter(|(name, _)| self.enabled.get(*name).copied().unwrap_or(false))
            .map(|(_, plugin)| Arc::clone(plugin))
            .collect()
    }

    /// Initialize all plugins
    pub async fn init_all(&mut self) -> Result<(), PluginError> {
        for (name, plugin) in &self.plugins {
            if self.enabled.get(name).copied().unwrap_or(false) {
                info!("Initializing plugin: {}", name);
                Arc::get_mut(plugin)
                    .ok_or_else(|| PluginError::InitFailed(format!("Cannot get mutable reference to {}", name)))?
                    .init()
                    .await?;
            }
        }
        Ok(())
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&mut self) -> Result<(), PluginError> {
        for (name, plugin) in &self.plugins {
            info!("Shutting down plugin: {}", name);
            Arc::get_mut(plugin)
                .ok_or_else(|| PluginError::InitFailed(format!("Cannot get mutable reference to {}", name)))?
                .shutdown()
                .await?;
        }
        Ok(())
    }

    /// Load plugins from directory (dynamic loading - future)
    pub async fn load_from_directory(&mut self, _path: impl AsRef<Path>) -> Result<(), PluginError> {
        // TODO: Implement dynamic plugin loading using libloading
        warn!("Dynamic plugin loading not yet implemented");
        Ok(())
    }

    /// List all plugins
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.plugins
            .values()
            .map(|p| p.metadata().clone())
            .collect()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
```

### 3.2 Example Plugins

#### 3.2.1 Validation Plugin (`plugins/example_validator/src/lib.rs`)

```rust
//! Example validation plugin

use llm_memory_graph::plugin::{Plugin, PluginBuilder, PluginContext, PluginError, PluginMetadata};
use async_trait::async_trait;
use regex::Regex;

pub struct ValidationPlugin {
    metadata: PluginMetadata,
    content_regex: Regex,
}

impl ValidationPlugin {
    pub fn new() -> Self {
        let metadata = PluginBuilder::new("content_validator", "1.0.0")
            .author("LLM DevOps Team")
            .description("Validates prompt content against rules")
            .capability("validation")
            .capability("content_filtering")
            .build();

        Self {
            metadata,
            content_regex: Regex::new(r"^[\w\s\.,!?-]+$").unwrap(),
        }
    }
}

#[async_trait]
impl Plugin for ValidationPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(&mut self) -> Result<(), PluginError> {
        tracing::info!("ValidationPlugin initialized");
        Ok(())
    }

    async fn before_create_node(&self, context: &PluginContext) -> Result<(), PluginError> {
        // Extract content from context
        if let Some(content) = context.data.get("content").and_then(|v| v.as_str()) {
            // Validate content length
            if content.len() > 10000 {
                return Err(PluginError::HookFailed(
                    "Content exceeds maximum length of 10000 characters".to_string()
                ));
            }

            // Validate content format
            if !self.content_regex.is_match(content) {
                return Err(PluginError::HookFailed(
                    "Content contains invalid characters".to_string()
                ));
            }
        }

        Ok(())
    }
}
```

#### 3.2.2 Enrichment Plugin (`plugins/example_enricher/src/lib.rs`)

```rust
//! Example enrichment plugin

use llm_memory_graph::plugin::{Plugin, PluginBuilder, PluginContext, PluginError, PluginMetadata};
use async_trait::async_trait;
use chrono::Utc;

pub struct EnrichmentPlugin {
    metadata: PluginMetadata,
}

impl EnrichmentPlugin {
    pub fn new() -> Self {
        let metadata = PluginBuilder::new("metadata_enricher", "1.0.0")
            .author("LLM DevOps Team")
            .description("Enriches nodes with additional metadata")
            .capability("enrichment")
            .capability("metadata")
            .build();

        Self { metadata }
    }
}

#[async_trait]
impl Plugin for EnrichmentPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn after_create_node(&self, context: &PluginContext) -> Result<(), PluginError> {
        // Add enrichment timestamp
        tracing::info!(
            "Node created at {} for operation: {}",
            Utc::now(),
            context.operation
        );

        // Could add additional metadata here
        // e.g., sentiment analysis, entity extraction, etc.

        Ok(())
    }

    async fn after_create_session(&self, context: &PluginContext) -> Result<(), PluginError> {
        tracing::info!("Session created: {:?}", context.data);
        Ok(())
    }
}
```

---

## 4. LLM-Registry Integration

### 4.1 Registry Client (`src/integrations/registry/client.rs`)

```rust
//! LLM-Registry client for metadata and version tracking

use serde::{Deserialize, Serialize};
use std::time::Duration;
use reqwest::Client;
use tracing::{info, error};

/// Registry client configuration
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout_secs: u64,
    pub retry_count: usize,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("REGISTRY_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            api_key: std::env::var("REGISTRY_API_KEY").ok(),
            timeout_secs: 30,
            retry_count: 3,
        }
    }
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub model_id: String,
    pub version: String,
    pub provider: String,
    pub parameters: ModelParameters,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    pub temperature: f64,
    pub max_tokens: Option<usize>,
    pub top_p: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
}

/// Session registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRegistration {
    pub session_id: String,
    pub model_id: String,
    pub metadata: serde_json::Value,
}

/// LLM-Registry client
pub struct RegistryClient {
    config: RegistryConfig,
    client: Client,
}

impl RegistryClient {
    pub fn new(config: RegistryConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;

        Ok(Self { config, client })
    }

    /// Register a session with the registry
    pub async fn register_session(
        &self,
        registration: SessionRegistration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/sessions", self.config.base_url);

        let mut request = self.client.post(&url).json(&registration);

        if let Some(api_key) = &self.config.api_key {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            error!("Failed to register session: {}", response.status());
            return Err(format!("Registry API error: {}", response.status()).into());
        }

        info!("Registered session {} with registry", registration.session_id);
        Ok(())
    }

    /// Get model metadata
    pub async fn get_model_metadata(
        &self,
        model_id: &str,
    ) -> Result<ModelMetadata, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/models/{}", self.config.base_url, model_id);

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            error!("Failed to get model metadata: {}", response.status());
            return Err(format!("Registry API error: {}", response.status()).into());
        }

        let metadata = response.json().await?;
        Ok(metadata)
    }

    /// Track token usage
    pub async fn track_usage(
        &self,
        session_id: &str,
        prompt_tokens: i64,
        completion_tokens: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Serialize)]
        struct UsageReport {
            session_id: String,
            prompt_tokens: i64,
            completion_tokens: i64,
            total_tokens: i64,
        }

        let url = format!("{}/api/v1/usage", self.config.base_url);

        let report = UsageReport {
            session_id: session_id.to_string(),
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        };

        let mut request = self.client.post(&url).json(&report);

        if let Some(api_key) = &self.config.api_key {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            error!("Failed to track usage: {}", response.status());
            return Err(format!("Registry API error: {}", response.status()).into());
        }

        Ok(())
    }

    /// Get usage statistics
    pub async fn get_usage_stats(
        &self,
        session_id: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/usage/{}", self.config.base_url, session_id);

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            error!("Failed to get usage stats: {}", response.status());
            return Err(format!("Registry API error: {}", response.status()).into());
        }

        let stats = response.json().await?;
        Ok(stats)
    }
}
```

---

## 5. Data-Vault Integration

### 5.1 Archival Service (`src/integrations/vault/archiver.rs`)

```rust
//! Data-Vault integration for secure archival and compliance

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::time::Duration as StdDuration;
use tracing::{info, warn, error};

/// Vault client configuration
#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub base_url: String,
    pub api_key: String,
    pub encryption_enabled: bool,
    pub compression_enabled: bool,
    pub timeout_secs: u64,
    pub batch_size: usize,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("VAULT_URL")
                .unwrap_or_else(|_| "http://localhost:9000".to_string()),
            api_key: std::env::var("VAULT_API_KEY")
                .expect("VAULT_API_KEY must be set"),
            encryption_enabled: true,
            compression_enabled: true,
            timeout_secs: 60,
            batch_size: 100,
        }
    }
}

/// Archive entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub id: String,
    pub session_id: String,
    pub data: serde_json::Value,
    pub archived_at: DateTime<Utc>,
    pub retention_days: i64,
    pub tags: Vec<String>,
}

/// Retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub policy_id: String,
    pub retention_days: i64,
    pub auto_delete: bool,
    pub compliance_level: ComplianceLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplianceLevel {
    Standard,
    Hipaa,
    Gdpr,
    Pci,
}

/// Data-Vault client
pub struct VaultClient {
    config: VaultConfig,
    client: Client,
}

impl VaultClient {
    pub fn new(config: VaultConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::builder()
            .timeout(StdDuration::from_secs(config.timeout_secs))
            .build()?;

        Ok(Self { config, client })
    }

    /// Archive a session
    pub async fn archive_session(
        &self,
        session_id: &str,
        data: serde_json::Value,
        retention_days: i64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let entry = ArchiveEntry {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            data,
            archived_at: Utc::now(),
            retention_days,
            tags: vec!["session".to_string()],
        };

        let url = format!("{}/api/v1/archive", self.config.base_url);

        let response = self.client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .header("X-Encryption-Enabled", self.config.encryption_enabled.to_string())
            .header("X-Compression-Enabled", self.config.compression_enabled.to_string())
            .json(&entry)
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Failed to archive session: {}", response.status());
            return Err(format!("Vault API error: {}", response.status()).into());
        }

        let archive_id = response.json::<serde_json::Value>().await?
            .get("archive_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing archive_id in response")?
            .to_string();

        info!("Archived session {} with ID {}", session_id, archive_id);
        Ok(archive_id)
    }

    /// Batch archive multiple sessions
    pub async fn batch_archive(
        &self,
        entries: Vec<ArchiveEntry>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/archive/batch", self.config.base_url);

        let response = self.client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .header("X-Encryption-Enabled", self.config.encryption_enabled.to_string())
            .header("X-Compression-Enabled", self.config.compression_enabled.to_string())
            .json(&entries)
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Failed to batch archive: {}", response.status());
            return Err(format!("Vault API error: {}", response.status()).into());
        }

        let archive_ids = response.json::<Vec<String>>().await?;
        info!("Batch archived {} entries", archive_ids.len());
        Ok(archive_ids)
    }

    /// Retrieve archived session
    pub async fn retrieve_session(
        &self,
        archive_id: &str,
    ) -> Result<ArchiveEntry, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/archive/{}", self.config.base_url, archive_id);

        let response = self.client
            .get(&url)
            .bearer_auth(&self.config.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Failed to retrieve archive: {}", response.status());
            return Err(format!("Vault API error: {}", response.status()).into());
        }

        let entry = response.json().await?;
        Ok(entry)
    }

    /// Delete archived session
    pub async fn delete_archive(
        &self,
        archive_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/archive/{}", self.config.base_url, archive_id);

        let response = self.client
            .delete(&url)
            .bearer_auth(&self.config.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Failed to delete archive: {}", response.status());
            return Err(format!("Vault API error: {}", response.status()).into());
        }

        info!("Deleted archive {}", archive_id);
        Ok(())
    }

    /// Create retention policy
    pub async fn create_retention_policy(
        &self,
        policy: RetentionPolicy,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/policies", self.config.base_url);

        let response = self.client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&policy)
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Failed to create retention policy: {}", response.status());
            return Err(format!("Vault API error: {}", response.status()).into());
        }

        let policy_id = response.json::<serde_json::Value>().await?
            .get("policy_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing policy_id in response")?
            .to_string();

        info!("Created retention policy {}", policy_id);
        Ok(policy_id)
    }

    /// Apply retention policy
    pub async fn apply_retention_policy(
        &self,
        policy_id: &str,
        archive_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/archive/{}/policy", self.config.base_url, archive_id);

        #[derive(Serialize)]
        struct PolicyApplication {
            policy_id: String,
        }

        let response = self.client
            .put(&url)
            .bearer_auth(&self.config.api_key)
            .json(&PolicyApplication { policy_id: policy_id.to_string() })
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Failed to apply retention policy: {}", response.status());
            return Err(format!("Vault API error: {}", response.status()).into());
        }

        info!("Applied policy {} to archive {}", policy_id, archive_id);
        Ok(())
    }
}

/// Automatic archival scheduler
pub struct ArchivalScheduler {
    vault_client: VaultClient,
    graph: Arc<crate::engine::AsyncMemoryGraph>,
    config: SchedulerConfig,
}

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub interval_hours: u64,
    pub archive_after_days: i64,
    pub retention_days: i64,
    pub batch_size: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            interval_hours: 24,
            archive_after_days: 30,
            retention_days: 365,
            batch_size: 100,
        }
    }
}

impl ArchivalScheduler {
    pub fn new(
        vault_client: VaultClient,
        graph: Arc<crate::engine::AsyncMemoryGraph>,
        config: SchedulerConfig,
    ) -> Self {
        Self {
            vault_client,
            graph,
            config,
        }
    }

    /// Start the archival scheduler
    pub async fn start(&self) -> tokio::task::JoinHandle<()> {
        let vault = self.vault_client.clone();
        let graph = Arc::clone(&self.graph);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                StdDuration::from_secs(config.interval_hours * 3600)
            );

            loop {
                interval.tick().await;

                info!("Running archival scheduler");

                // Find sessions older than archive_after_days
                let cutoff_date = Utc::now() - Duration::days(config.archive_after_days);

                // TODO: Implement query for old sessions
                // For now, this is a placeholder

                warn!("Archival scheduler iteration complete");
            }
        })
    }
}
```

---

## 6. Enhanced Prometheus Metrics

### 6.1 Extended Metrics (`src/observatory/prometheus.rs`)

Add to the existing Prometheus implementation:

```rust
// Add these new metrics to the existing PrometheusMetrics struct

/// gRPC-specific metrics
pub grpc_requests_total: IntCounterVec,
pub grpc_request_duration: HistogramVec,
pub grpc_active_streams: IntGauge,

/// Plugin metrics
pub plugin_executions_total: IntCounterVec,
pub plugin_duration: HistogramVec,
pub plugin_errors_total: IntCounterVec,

/// Integration metrics
pub registry_calls_total: IntCounterVec,
pub vault_archives_total: IntCounter,
pub vault_retrievals_total: IntCounter,
pub vault_errors_total: IntCounter,

// In the implementation:

impl PrometheusMetrics {
    pub fn new(registry: &Registry) -> Result<Self> {
        // ... existing metrics

        // gRPC metrics
        let grpc_requests_total = IntCounterVec::new(
            Opts::new(
                "memory_graph_grpc_requests_total",
                "Total number of gRPC requests by method"
            ),
            &["method", "status"],
        )?;
        registry.register(Box::new(grpc_requests_total.clone()))?;

        let grpc_request_duration = HistogramVec::new(
            HistogramOpts::new(
                "memory_graph_grpc_request_duration_seconds",
                "gRPC request duration in seconds"
            ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
            &["method"],
        )?;
        registry.register(Box::new(grpc_request_duration.clone()))?;

        let grpc_active_streams = IntGauge::new(
            "memory_graph_grpc_active_streams",
            "Number of active gRPC streams"
        )?;
        registry.register(Box::new(grpc_active_streams.clone()))?;

        // Plugin metrics
        let plugin_executions_total = IntCounterVec::new(
            Opts::new(
                "memory_graph_plugin_executions_total",
                "Total plugin executions by name and hook"
            ),
            &["plugin", "hook"],
        )?;
        registry.register(Box::new(plugin_executions_total.clone()))?;

        let plugin_duration = HistogramVec::new(
            HistogramOpts::new(
                "memory_graph_plugin_duration_seconds",
                "Plugin execution duration"
            ).buckets(vec![0.001, 0.01, 0.1, 1.0]),
            &["plugin", "hook"],
        )?;
        registry.register(Box::new(plugin_duration.clone()))?;

        let plugin_errors_total = IntCounterVec::new(
            Opts::new(
                "memory_graph_plugin_errors_total",
                "Total plugin errors"
            ),
            &["plugin", "error_type"],
        )?;
        registry.register(Box::new(plugin_errors_total.clone()))?;

        // Integration metrics
        let registry_calls_total = IntCounterVec::new(
            Opts::new(
                "memory_graph_registry_calls_total",
                "Total LLM-Registry API calls"
            ),
            &["operation", "status"],
        )?;
        registry.register(Box::new(registry_calls_total.clone()))?;

        let vault_archives_total = IntCounter::new(
            "memory_graph_vault_archives_total",
            "Total sessions archived to Data-Vault"
        )?;
        registry.register(Box::new(vault_archives_total.clone()))?;

        let vault_retrievals_total = IntCounter::new(
            "memory_graph_vault_retrievals_total",
            "Total sessions retrieved from Data-Vault"
        )?;
        registry.register(Box::new(vault_retrievals_total.clone()))?;

        let vault_errors_total = IntCounter::new(
            "memory_graph_vault_errors_total",
            "Total Data-Vault errors"
        )?;
        registry.register(Box::new(vault_errors_total.clone()))?;

        Ok(Self {
            // ... existing fields
            grpc_requests_total,
            grpc_request_duration,
            grpc_active_streams,
            plugin_executions_total,
            plugin_duration,
            plugin_errors_total,
            registry_calls_total,
            vault_archives_total,
            vault_retrievals_total,
            vault_errors_total,
        })
    }
}
```

---

## 7. Deployment Architecture

### 7.1 Docker Configuration

#### `deploy/docker/Dockerfile`

```dockerfile
# Multi-stage build for LLM-Memory-Graph gRPC server

# Build stage
FROM rust:1.75-slim as builder

WORKDIR /usr/src/llm-memory-graph

# Install dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libprotobuf-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY proto ./proto
COPY build.rs ./

# Copy source
COPY src ./src

# Build release binary
RUN cargo build --release --bin server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1001 -s /bin/bash appuser

# Create data directory
RUN mkdir -p /data && chown appuser:appuser /data

# Copy binary from builder
COPY --from=builder /usr/src/llm-memory-graph/target/release/server /usr/local/bin/server

# Switch to app user
USER appuser

# Set environment variables
ENV DB_PATH=/data
ENV GRPC_HOST=0.0.0.0
ENV GRPC_PORT=50051
ENV METRICS_PORT=9090
ENV RUST_LOG=info

# Expose ports
EXPOSE 50051 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD grpc_health_probe -addr=:50051 || exit 1

# Run server
CMD ["server"]
```

#### `deploy/docker/docker-compose.yml`

```yaml
version: '3.8'

services:
  memory-graph:
    build:
      context: ../..
      dockerfile: deploy/docker/Dockerfile
    container_name: llm-memory-graph
    ports:
      - "50051:50051"  # gRPC
      - "9090:9090"    # Metrics
    volumes:
      - memory-graph-data:/data
      - ./plugins:/plugins:ro
    environment:
      - DB_PATH=/data
      - GRPC_HOST=0.0.0.0
      - GRPC_PORT=50051
      - METRICS_PORT=9090
      - RUST_LOG=info
      - PLUGIN_DIRS=/plugins
      - REGISTRY_URL=http://llm-registry:8080
      - REGISTRY_API_KEY=${REGISTRY_API_KEY}
      - VAULT_URL=http://data-vault:9000
      - VAULT_API_KEY=${VAULT_API_KEY}
    networks:
      - llm-network
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "grpc_health_probe", "-addr=:50051"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  prometheus:
    image: prom/prometheus:latest
    container_name: llm-memory-graph-prometheus
    ports:
      - "9091:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    networks:
      - llm-network
    restart: unless-stopped

  grafana:
    image: grafana/grafana:latest
    container_name: llm-memory-graph-grafana
    ports:
      - "3000:3000"
    volumes:
      - ../../grafana:/etc/grafana/provisioning/dashboards
      - ./grafana-datasources.yml:/etc/grafana/provisioning/datasources/datasources.yml
      - grafana-data:/var/lib/grafana
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false
    networks:
      - llm-network
    restart: unless-stopped
    depends_on:
      - prometheus

volumes:
  memory-graph-data:
  prometheus-data:
  grafana-data:

networks:
  llm-network:
    driver: bridge
```

### 7.2 Kubernetes Deployment

#### `deploy/kubernetes/deployment.yaml`

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: llm-memory-graph

---
apiVersion: v1
kind: ConfigMap
metadata:
  name: memory-graph-config
  namespace: llm-memory-graph
data:
  GRPC_HOST: "0.0.0.0"
  GRPC_PORT: "50051"
  METRICS_PORT: "9090"
  RUST_LOG: "info"

---
apiVersion: v1
kind: Secret
metadata:
  name: memory-graph-secrets
  namespace: llm-memory-graph
type: Opaque
stringData:
  REGISTRY_API_KEY: "your-registry-api-key"
  VAULT_API_KEY: "your-vault-api-key"

---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: memory-graph-data
  namespace: llm-memory-graph
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 100Gi
  storageClassName: standard

---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: memory-graph
  namespace: llm-memory-graph
  labels:
    app: memory-graph
    version: v1.0.0
spec:
  replicas: 3
  selector:
    matchLabels:
      app: memory-graph
  template:
    metadata:
      labels:
        app: memory-graph
        version: v1.0.0
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      containers:
      - name: memory-graph
        image: ghcr.io/globalbusinessadvisors/llm-memory-graph:latest
        imagePullPolicy: Always
        ports:
        - name: grpc
          containerPort: 50051
          protocol: TCP
        - name: metrics
          containerPort: 9090
          protocol: TCP
        env:
        - name: DB_PATH
          value: "/data"
        envFrom:
        - configMapRef:
            name: memory-graph-config
        - secretRef:
            name: memory-graph-secrets
        volumeMounts:
        - name: data
          mountPath: /data
        - name: plugins
          mountPath: /plugins
          readOnly: true
        resources:
          requests:
            memory: "2Gi"
            cpu: "1000m"
          limits:
            memory: "4Gi"
            cpu: "2000m"
        livenessProbe:
          grpc:
            port: 50051
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          grpc:
            port: 50051
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 2
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: memory-graph-data
      - name: plugins
        configMap:
          name: memory-graph-plugins

---
apiVersion: v1
kind: Service
metadata:
  name: memory-graph
  namespace: llm-memory-graph
  labels:
    app: memory-graph
spec:
  type: LoadBalancer
  ports:
  - name: grpc
    port: 50051
    targetPort: 50051
    protocol: TCP
  - name: metrics
    port: 9090
    targetPort: 9090
    protocol: TCP
  selector:
    app: memory-graph

---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: memory-graph-hpa
  namespace: llm-memory-graph
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: memory-graph
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  - type: Pods
    pods:
      metric:
        name: grpc_requests_per_second
      target:
        type: AverageValue
        averageValue: "1000"
```

---

## 8. Implementation Roadmap

### Phase 1: gRPC Service (Weeks 1-2)

**Week 1: Foundation**
- ✅ Define protobuf schema
- ✅ Implement build.rs for code generation
- ✅ Create basic service structure
- ✅ Implement session management endpoints
- ✅ Add basic error handling

**Week 2: Core Operations**
- ✅ Implement node CRUD operations
- ✅ Implement edge operations
- ✅ Add query operations
- ✅ Implement streaming queries
- ✅ Add health and metrics endpoints

### Phase 2: Plugin System (Weeks 3-4)

**Week 3: Plugin Framework**
- ✅ Design plugin trait and hooks
- ✅ Implement plugin manager
- ✅ Create plugin lifecycle management
- ✅ Add plugin configuration system
- ✅ Document plugin API

**Week 4: Reference Implementations**
- ✅ Create validation plugin
- ✅ Create enrichment plugin
- ✅ Add plugin testing framework
- ✅ Create plugin SDK documentation
- ✅ Integration with gRPC service

### Phase 3: Ecosystem Integration (Weeks 5-6)

**Week 5: LLM-Registry**
- ✅ Implement Registry client
- ✅ Add session registration
- ✅ Add model metadata tracking
- ✅ Implement usage tracking
- ✅ Add retry and error handling

**Week 6: Data-Vault**
- ✅ Implement Vault client
- ✅ Add archival functionality
- ✅ Implement retention policies
- ✅ Create archival scheduler
- ✅ Add batch operations

### Phase 4: Deployment & Testing (Weeks 7-8)

**Week 7: Deployment**
- ✅ Create Dockerfile
- ✅ Create docker-compose configuration
- ✅ Create Kubernetes manifests
- ✅ Add Helm charts
- ✅ Create deployment documentation

**Week 8: Testing & Documentation**
- ✅ Write integration tests
- ✅ Performance testing
- ✅ Security audit
- ✅ Complete documentation
- ✅ Create runbooks

---

## 9. Testing Strategy

### 9.1 Unit Tests (Target: 150+ tests)

```rust
// gRPC service tests
#[tokio::test]
async fn test_grpc_create_session() { }

#[tokio::test]
async fn test_grpc_add_prompt() { }

#[tokio::test]
async fn test_grpc_stream_query() { }

// Plugin tests
#[tokio::test]
async fn test_plugin_registration() { }

#[tokio::test]
async fn test_plugin_hooks() { }

#[tokio::test]
async fn test_plugin_lifecycle() { }

// Integration tests
#[tokio::test]
async fn test_registry_integration() { }

#[tokio::test]
async fn test_vault_integration() { }
```

### 9.2 Integration Tests (Target: 30 tests)

```rust
#[tokio::test]
async fn test_end_to_end_grpc_workflow() {
    // 1. Start gRPC server
    // 2. Create session
    // 3. Add prompts and responses
    // 4. Query data
    // 5. Verify results
}

#[tokio::test]
async fn test_plugin_validation_workflow() {
    // 1. Load validation plugin
    // 2. Attempt invalid operation
    // 3. Verify rejection
}

#[tokio::test]
async fn test_archival_workflow() {
    // 1. Create session with data
    // 2. Archive to vault
    // 3. Retrieve from vault
    // 4. Verify integrity
}
```

### 9.3 Performance Tests

```rust
#[tokio::test]
async fn test_grpc_concurrent_requests() {
    // Test 1000 concurrent gRPC requests
    // Target: <100ms p95 latency
}

#[tokio::test]
async fn test_streaming_query_performance() {
    // Test streaming 10,000 nodes
    // Target: <5s total time
}

#[tokio::test]
async fn test_plugin_overhead() {
    // Measure plugin execution overhead
    // Target: <10ms per hook
}
```

### 9.4 E2E Tests (5 tests)

```rust
#[tokio::test]
async fn test_complete_llm_workflow() {
    // Full workflow from prompt to archive
    // Including Registry and Vault integration
}

#[tokio::test]
async fn test_multi_tenant_isolation() {
    // Verify session isolation across tenants
}

#[tokio::test]
async fn test_disaster_recovery() {
    // Test backup and restore workflow
}

#[tokio::test]
async fn test_compliance_workflow() {
    // Test GDPR/HIPAA compliance features
}

#[tokio::test]
async fn test_monitoring_integration() {
    // Verify metrics and tracing
}
```

---

## 10. Security & Compliance

### 10.1 Authentication & Authorization

```rust
// gRPC interceptor for authentication
pub struct AuthInterceptor {
    api_keys: Arc<HashSet<String>>,
}

impl tonic::service::Interceptor for AuthInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let token = request.metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("Missing authorization header"))?;

        if !self.api_keys.contains(token) {
            return Err(Status::unauthenticated("Invalid API key"));
        }

        Ok(request)
    }
}
```

### 10.2 TLS Configuration

```rust
// Server TLS setup
let cert = tokio::fs::read("certs/server.crt").await?;
let key = tokio::fs::read("certs/server.key").await?;

let identity = tonic::transport::Identity::from_pem(cert, key);

let server = Server::builder()
    .tls_config(ServerTlsConfig::new().identity(identity))?
    .add_service(service)
    .serve(addr);
```

### 10.3 Data Encryption

- **At Rest**: Sled database encryption
- **In Transit**: TLS 1.3 for gRPC
- **Vault Integration**: AES-256-GCM encryption

---

## 11. Performance Targets

| Metric | Target | Monitoring |
|--------|--------|------------|
| **gRPC Latency (p95)** | <50ms | Prometheus histogram |
| **Streaming Throughput** | >10,000 nodes/sec | Custom metric |
| **Plugin Overhead** | <10ms per hook | Plugin duration histogram |
| **Concurrent Connections** | 10,000+ | Active connections gauge |
| **Memory Usage** | <4GB per instance | cAdvisor/Kubernetes |
| **CPU Usage** | <70% average | cAdvisor/Kubernetes |

---

## 12. Success Metrics

### Technical Metrics

- ✅ 100% gRPC API coverage
- ✅ 90%+ test coverage
- ✅ <50ms p95 latency
- ✅ 99.9% uptime
- ✅ Zero data loss
- ✅ Support 10,000 concurrent connections

### Business Metrics

- ✅ Production deployment in 3+ environments
- ✅ 5+ custom plugins developed
- ✅ Integration with 3+ LLM providers
- ✅ 1M+ archived sessions
- ✅ Complete documentation and runbooks

---

## 13. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| **gRPC Performance** | Extensive benchmarking, load testing |
| **Plugin Security** | Sandboxing, code review, capability limits |
| **Data Loss** | Regular backups, vault archival |
| **Integration Failures** | Circuit breakers, retry logic, fallbacks |
| **Scalability Issues** | Horizontal scaling, connection pooling |

---

## Appendix A: API Reference

Complete gRPC API documentation will be generated from protobuf definitions and available at:
- Swagger UI: `http://localhost:50051/swagger`
- ReDoc: `http://localhost:50051/redoc`

---

## Appendix B: Plugin Development Guide

See `docs/PLUGIN_DEVELOPMENT.md` for complete guide on creating custom plugins.

---

## Appendix C: Deployment Runbook

See `docs/DEPLOYMENT_RUNBOOK.md` for step-by-step production deployment instructions.

---

**Document Version**: 1.0
**Last Updated**: 2025-11-07
**Next Review**: 2025-12-07
