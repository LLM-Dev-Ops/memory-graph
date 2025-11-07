//! Main gRPC service implementation
//!
//! This module implements the MemoryGraphService defined in the protobuf schema.
//! It provides all CRUD operations, query interfaces, and streaming endpoints.

use crate::engine::AsyncMemoryGraph;
use crate::grpc::converters::*;
use crate::grpc::proto::memory_graph_service_server::MemoryGraphService;
use crate::grpc::proto::*;
use crate::observatory::prometheus::PrometheusMetrics;
use std::sync::Arc;
use std::time::Instant as StdInstant;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use tracing::{error, info, instrument, warn};

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Server host address
    pub host: String,
    /// Server port
    pub port: u16,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Enable gRPC reflection
    pub enable_reflection: bool,
    /// Enable health checks
    pub enable_health: bool,
    /// Server start time for uptime calculation
    pub start_time: StdInstant,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: crate::grpc::DEFAULT_GRPC_PORT,
            max_connections: 1000,
            request_timeout_ms: crate::grpc::DEFAULT_REQUEST_TIMEOUT_MS,
            enable_reflection: true,
            enable_health: true,
            start_time: StdInstant::now(),
        }
    }
}

/// gRPC service implementation for MemoryGraph
pub struct MemoryGraphServiceImpl {
    /// Core async memory graph
    graph: Arc<AsyncMemoryGraph>,
    /// Prometheus metrics (optional)
    metrics: Option<Arc<PrometheusMetrics>>,
    /// Service configuration
    config: ServiceConfig,
}

impl MemoryGraphServiceImpl {
    /// Create a new service instance
    pub fn new(
        graph: Arc<AsyncMemoryGraph>,
        metrics: Option<Arc<PrometheusMetrics>>,
        config: ServiceConfig,
    ) -> Self {
        Self {
            graph,
            metrics,
            config,
        }
    }

    /// Record gRPC request metrics
    fn record_request(&self, method: &str, latency_secs: f64, success: bool) {
        if let Some(metrics) = &self.metrics {
            // Record in existing Prometheus metrics
            if success {
                metrics.record_query_duration(latency_secs);
            }
            // Additional gRPC-specific metrics would go here
            tracing::debug!(
                method = method,
                latency_ms = latency_secs * 1000.0,
                success = success,
                "gRPC request completed"
            );
        }
    }
}

#[tonic::async_trait]
impl MemoryGraphService for MemoryGraphServiceImpl {
    // ========================================================================
    // Session Management
    // ========================================================================

    #[instrument(skip(self))]
    async fn create_session(
        &self,
        request: Request<CreateSessionRequest>,
    ) -> Result<Response<Session>, Status> {
        let start = StdInstant::now();
        let req = request.into_inner();

        info!("Creating session with metadata: {:?}", req.metadata);

        let session = if req.metadata.is_empty() {
            self.graph.create_session().await
        } else {
            self.graph.create_session_with_metadata(req.metadata).await
        }
        .map_err(error_to_status)?;

        let proto_session = session_to_proto(session);
        self.record_request("create_session", start.elapsed().as_secs_f64(), true);

        Ok(Response::new(proto_session))
    }

    #[instrument(skip(self))]
    async fn get_session(
        &self,
        request: Request<GetSessionRequest>,
    ) -> Result<Response<Session>, Status> {
        let start = StdInstant::now();
        let req = request.into_inner();

        let session_id = parse_session_id(&req.session_id)?;
        let session = self.graph.get_session(session_id).await.map_err(error_to_status)?;

        let proto_session = session_to_proto(session);
        self.record_request("get_session", start.elapsed().as_secs_f64(), true);

        Ok(Response::new(proto_session))
    }

    #[instrument(skip(self))]
    async fn delete_session(
        &self,
        request: Request<DeleteSessionRequest>,
    ) -> Result<Response<()>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement session deletion
        warn!("delete_session not yet implemented");
        Err(Status::unimplemented("Session deletion not yet implemented"))
    }

    #[instrument(skip(self))]
    async fn list_sessions(
        &self,
        request: Request<ListSessionsRequest>,
    ) -> Result<Response<ListSessionsResponse>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement session listing with pagination
        warn!("list_sessions not yet implemented");
        Err(Status::unimplemented("Session listing not yet implemented"))
    }

    // ========================================================================
    // Node Operations
    // ========================================================================

    #[instrument(skip(self))]
    async fn create_node(
        &self,
        request: Request<CreateNodeRequest>,
    ) -> Result<Response<Node>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement generic node creation
        warn!("create_node not yet implemented");
        Err(Status::unimplemented("Generic node creation not yet implemented"))
    }

    #[instrument(skip(self))]
    async fn get_node(
        &self,
        request: Request<GetNodeRequest>,
    ) -> Result<Response<Node>, Status> {
        let start = StdInstant::now();
        let req = request.into_inner();

        let node_id = parse_node_id(&req.node_id)?;
        let node = self.graph
            .get_node(&node_id)
            .await
            .map_err(error_to_status)?
            .ok_or_else(|| Status::not_found(format!("Node {} not found", node_id)))?;

        let proto_node = node_to_proto(node);
        self.record_request("get_node", start.elapsed().as_secs_f64(), true);

        Ok(Response::new(proto_node))
    }

    #[instrument(skip(self))]
    async fn update_node(
        &self,
        request: Request<UpdateNodeRequest>,
    ) -> Result<Response<Node>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement node update
        warn!("update_node not yet implemented");
        Err(Status::unimplemented("Node update not yet implemented"))
    }

    #[instrument(skip(self))]
    async fn delete_node(
        &self,
        request: Request<DeleteNodeRequest>,
    ) -> Result<Response<()>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement node deletion
        warn!("delete_node not yet implemented");
        Err(Status::unimplemented("Node deletion not yet implemented"))
    }

    #[instrument(skip(self))]
    async fn batch_create_nodes(
        &self,
        request: Request<BatchCreateNodesRequest>,
    ) -> Result<Response<BatchCreateNodesResponse>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement batch node creation
        warn!("batch_create_nodes not yet implemented");
        Err(Status::unimplemented("Batch node creation not yet implemented"))
    }

    #[instrument(skip(self))]
    async fn batch_get_nodes(
        &self,
        request: Request<BatchGetNodesRequest>,
    ) -> Result<Response<BatchGetNodesResponse>, Status> {
        let start = StdInstant::now();
        let req = request.into_inner();

        let node_ids: Result<Vec<_>, _> = req
            .node_ids
            .iter()
            .map(|id| parse_node_id(id))
            .collect();
        let node_ids = node_ids?;

        let nodes = self.graph
            .get_nodes_batch(node_ids)
            .await
            .map_err(error_to_status)?;

        let proto_nodes: Vec<Node> = nodes
            .into_iter()
            .filter_map(|opt_node| opt_node.map(node_to_proto))
            .collect();

        self.record_request("batch_get_nodes", start.elapsed().as_secs_f64(), true);

        Ok(Response::new(BatchGetNodesResponse {
            nodes: proto_nodes,
        }))
    }

    // ========================================================================
    // Edge Operations
    // ========================================================================

    #[instrument(skip(self))]
    async fn create_edge(
        &self,
        request: Request<CreateEdgeRequest>,
    ) -> Result<Response<Edge>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement edge creation
        warn!("create_edge not yet implemented");
        Err(Status::unimplemented("Edge creation not yet implemented"))
    }

    #[instrument(skip(self))]
    async fn get_edges(
        &self,
        request: Request<GetEdgesRequest>,
    ) -> Result<Response<GetEdgesResponse>, Status> {
        let start = StdInstant::now();
        let req = request.into_inner();

        let node_id = parse_node_id(&req.node_id)?;

        // Get outgoing edges by default, or as specified
        let edges = match req.direction {
            Some(dir) if dir == EdgeDirection::EdgeDirectionIncoming as i32 => {
                self.graph.get_incoming_edges(&node_id).await
            }
            Some(dir) if dir == EdgeDirection::EdgeDirectionOutgoing as i32 => {
                self.graph.get_outgoing_edges(&node_id).await
            }
            Some(dir) if dir == EdgeDirection::EdgeDirectionBoth as i32 => {
                // Get both incoming and outgoing
                let mut outgoing = self.graph.get_outgoing_edges(&node_id).await.map_err(error_to_status)?;
                let mut incoming = self.graph.get_incoming_edges(&node_id).await.map_err(error_to_status)?;
                outgoing.append(&mut incoming);
                Ok(outgoing)
            }
            _ => self.graph.get_outgoing_edges(&node_id).await,
        }
        .map_err(error_to_status)?;

        let proto_edges: Vec<Edge> = edges.into_iter().map(edge_to_proto).collect();
        self.record_request("get_edges", start.elapsed().as_secs_f64(), true);

        Ok(Response::new(GetEdgesResponse { edges: proto_edges }))
    }

    #[instrument(skip(self))]
    async fn delete_edge(
        &self,
        request: Request<DeleteEdgeRequest>,
    ) -> Result<Response<()>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement edge deletion
        warn!("delete_edge not yet implemented");
        Err(Status::unimplemented("Edge deletion not yet implemented"))
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    #[instrument(skip(self))]
    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement query operation
        warn!("query not yet implemented");
        Err(Status::unimplemented("Query operation not yet implemented"))
    }

    #[instrument(skip(self))]
    async fn stream_query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<Self::StreamQueryStream>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement streaming query
        warn!("stream_query not yet implemented");
        Err(Status::unimplemented("Stream query not yet implemented"))
    }

    // ========================================================================
    // Prompt & Response Operations
    // ========================================================================

    #[instrument(skip(self))]
    async fn add_prompt(
        &self,
        request: Request<AddPromptRequest>,
    ) -> Result<Response<PromptNode>, Status> {
        let start = StdInstant::now();
        let req = request.into_inner();

        let session_id = parse_session_id(&req.session_id)?;
        let metadata = req.metadata.map(proto_to_prompt_metadata);

        let prompt_id = self.graph
            .add_prompt(session_id, req.content, metadata)
            .await
            .map_err(error_to_status)?;

        // Retrieve the created prompt
        let node = self.graph
            .get_node(&prompt_id)
            .await
            .map_err(error_to_status)?
            .ok_or_else(|| Status::internal("Failed to retrieve created prompt"))?;

        let proto_prompt = match node {
            crate::types::Node::Prompt(p) => prompt_node_to_proto(p),
            _ => return Err(Status::internal("Unexpected node type")),
        };

        self.record_request("add_prompt", start.elapsed().as_secs_f64(), true);
        Ok(Response::new(proto_prompt))
    }

    #[instrument(skip(self))]
    async fn add_response(
        &self,
        request: Request<AddResponseRequest>,
    ) -> Result<Response<ResponseNode>, Status> {
        let start = StdInstant::now();
        let req = request.into_inner();

        let prompt_id = parse_node_id(&req.prompt_id)?;
        let token_usage = req.token_usage
            .ok_or_else(|| Status::invalid_argument("Missing token_usage"))?;
        let token_usage = proto_to_token_usage(token_usage);
        let metadata = req.metadata.map(proto_to_response_metadata);

        let response_id = self.graph
            .add_response(prompt_id, req.content, token_usage, metadata)
            .await
            .map_err(error_to_status)?;

        // Retrieve the created response
        let node = self.graph
            .get_node(&response_id)
            .await
            .map_err(error_to_status)?
            .ok_or_else(|| Status::internal("Failed to retrieve created response"))?;

        let proto_response = match node {
            crate::types::Node::Response(r) => response_node_to_proto(r),
            _ => return Err(Status::internal("Unexpected node type")),
        };

        self.record_request("add_response", start.elapsed().as_secs_f64(), true);
        Ok(Response::new(proto_response))
    }

    #[instrument(skip(self))]
    async fn add_tool_invocation(
        &self,
        request: Request<AddToolInvocationRequest>,
    ) -> Result<Response<ToolInvocationNode>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement tool invocation
        warn!("add_tool_invocation not yet implemented");
        Err(Status::unimplemented("Tool invocation not yet implemented"))
    }

    // ========================================================================
    // Template Operations
    // ========================================================================

    #[instrument(skip(self))]
    async fn create_template(
        &self,
        request: Request<CreateTemplateRequest>,
    ) -> Result<Response<TemplateNode>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement template creation
        warn!("create_template not yet implemented");
        Err(Status::unimplemented("Template creation not yet implemented"))
    }

    #[instrument(skip(self))]
    async fn instantiate_template(
        &self,
        request: Request<InstantiateTemplateRequest>,
    ) -> Result<Response<PromptNode>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement template instantiation
        warn!("instantiate_template not yet implemented");
        Err(Status::unimplemented("Template instantiation not yet implemented"))
    }

    // ========================================================================
    // Streaming Operations
    // ========================================================================

    #[instrument(skip(self))]
    async fn stream_events(
        &self,
        request: Request<StreamEventsRequest>,
    ) -> Result<Response<Self::StreamEventsStream>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement event streaming
        warn!("stream_events not yet implemented");
        Err(Status::unimplemented("Event streaming not yet implemented"))
    }

    #[instrument(skip(self))]
    async fn subscribe_to_session(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeToSessionStream>, Status> {
        let _start = StdInstant::now();
        let _req = request.into_inner();

        // TODO: Implement session subscription
        warn!("subscribe_to_session not yet implemented");
        Err(Status::unimplemented("Session subscription not yet implemented"))
    }

    // ========================================================================
    // Health & Metrics
    // ========================================================================

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
        let stats = self.graph.stats().await.map_err(error_to_status)?;

        // Get Prometheus metrics if available
        let (active_sessions, avg_write_latency_ms, avg_read_latency_ms) =
            if let Some(graph_metrics) = self.graph.get_metrics() {
                (
                    graph_metrics.sessions_created as i64,
                    graph_metrics.avg_write_latency_ms,
                    graph_metrics.avg_read_latency_ms,
                )
            } else {
                (0, 0.0, 0.0)
            };

        Ok(Response::new(MetricsResponse {
            total_nodes: stats.node_count as i64,
            total_edges: stats.edge_count as i64,
            total_sessions: stats.session_count as i64,
            active_sessions,
            avg_write_latency_ms,
            avg_read_latency_ms,
            requests_per_second: 0, // TODO: Calculate from metrics
        }))
    }
}
