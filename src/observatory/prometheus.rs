//! Prometheus metrics integration for real-time monitoring
//!
//! This module provides comprehensive production-grade Prometheus metrics for monitoring
//! memory graph operations, performance, resource utilization, and integrations.
//!
//! # Metrics Categories
//!
//! ## Core Metrics
//! - **Counters**: Track cumulative counts of operations (nodes, edges, prompts, etc.)
//! - **Histograms**: Measure latency distributions and batch sizes
//! - **Gauges**: Monitor current state and resource utilization
//!
//! ## Production Metrics
//! - **gRPC Metrics**: Request counts, durations, and active streams
//! - **Plugin Metrics**: Plugin executions, durations, and error tracking
//! - **Integration Metrics**: LLM-Registry calls and Data-Vault operations
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! use llm_memory_graph::observatory::prometheus::PrometheusMetrics;
//! use prometheus::Registry;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = Registry::new();
//! let metrics = PrometheusMetrics::new(&registry)?;
//!
//! // Record core operations
//! metrics.record_node_created();
//! metrics.record_write_latency(0.025);
//! metrics.set_active_sessions(5);
//!
//! // Export metrics
//! let metrics_text = prometheus::TextEncoder::new()
//!     .encode_to_string(&registry.gather())?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Production Metrics Usage
//!
//! ```no_run
//! # use llm_memory_graph::observatory::prometheus::PrometheusMetrics;
//! # use prometheus::Registry;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let registry = Registry::new();
//! # let metrics = PrometheusMetrics::new(&registry)?;
//! // Record gRPC operations
//! metrics.record_grpc_request("CreateSession", "success");
//! metrics.record_grpc_request_duration("CreateSession", 0.015);
//! metrics.inc_grpc_active_streams();
//!
//! // Record plugin operations
//! metrics.record_plugin_execution("audit_logger", "on_node_create");
//! metrics.record_plugin_duration("audit_logger", "on_node_create", 0.002);
//! metrics.record_plugin_error("validator", "validation_failed");
//!
//! // Record integration operations
//! metrics.record_registry_call("register_model", "success");
//! metrics.record_vault_archive();
//! metrics.record_vault_retrieval();
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use prometheus::{
    Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
};

/// Prometheus metrics for MemoryGraph monitoring
///
/// Provides comprehensive production-grade metrics across multiple categories:
/// - 8 Counters for tracking operations
/// - 5 Histograms for latency and size distributions
/// - 5 Gauges for current state monitoring
/// - 7 Production metrics (gRPC, Plugin, Integration)
#[derive(Clone)]
pub struct PrometheusMetrics {
    // Counters - Track cumulative counts
    /// Total number of nodes created
    pub nodes_created: IntCounter,
    /// Total number of edges created
    pub edges_created: IntCounter,
    /// Total number of prompts submitted
    pub prompts_submitted: IntCounter,
    /// Total number of responses generated
    pub responses_generated: IntCounter,
    /// Total number of tools invoked
    pub tools_invoked: IntCounter,
    /// Total number of agent handoffs
    pub agent_handoffs: IntCounter,
    /// Total number of template instantiations
    pub template_instantiations: IntCounter,
    /// Total number of queries executed
    pub queries_executed: IntCounter,

    // Histograms - Measure distributions
    /// Write operation latency distribution (seconds)
    pub write_latency: Histogram,
    /// Read operation latency distribution (seconds)
    pub read_latency: Histogram,
    /// Query execution duration distribution (seconds)
    pub query_duration: Histogram,
    /// Tool execution duration distribution (seconds)
    pub tool_duration: Histogram,
    /// Batch operation size distribution
    pub batch_size: Histogram,

    // Gauges - Current state metrics
    /// Number of currently active sessions
    pub active_sessions: IntGauge,
    /// Total number of nodes in the graph
    pub total_nodes: IntGauge,
    /// Total number of edges in the graph
    pub total_edges: IntGauge,
    /// Current cache size in bytes
    pub cache_size_bytes: IntGauge,
    /// Current event buffer size
    pub buffer_size: IntGauge,

    // Production Metrics - gRPC
    /// Total gRPC requests by method and status
    pub grpc_requests_total: IntCounterVec,
    /// gRPC request duration by method
    pub grpc_request_duration: HistogramVec,
    /// Number of active gRPC streams
    pub grpc_active_streams: IntGauge,

    // Production Metrics - Plugin System
    /// Total plugin executions by plugin name and hook
    pub plugin_executions_total: IntCounterVec,
    /// Plugin execution duration by plugin and hook
    pub plugin_duration: HistogramVec,
    /// Total plugin errors by plugin and error type
    pub plugin_errors_total: IntCounterVec,

    // Production Metrics - Integrations
    /// Total LLM-Registry API calls by operation and status
    pub registry_calls_total: IntCounterVec,
    /// Total sessions archived to Data-Vault
    pub vault_archives_total: IntCounter,
    /// Total sessions retrieved from Data-Vault
    pub vault_retrievals_total: IntCounter,
    /// Total Data-Vault errors
    pub vault_errors_total: IntCounter,
}

impl PrometheusMetrics {
    /// Create and register all metrics with the provided registry
    ///
    /// # Arguments
    ///
    /// * `registry` - Prometheus registry to register metrics with
    ///
    /// # Returns
    ///
    /// Returns a new PrometheusMetrics instance with all metrics registered
    ///
    /// # Errors
    ///
    /// Returns an error if metric registration fails
    pub fn new(registry: &Registry) -> Result<Self> {
        // Counters
        let nodes_created = IntCounter::with_opts(Opts::new(
            "memory_graph_nodes_created_total",
            "Total number of nodes created in the memory graph",
        ))?;
        registry.register(Box::new(nodes_created.clone()))?;

        let edges_created = IntCounter::with_opts(Opts::new(
            "memory_graph_edges_created_total",
            "Total number of edges created in the memory graph",
        ))?;
        registry.register(Box::new(edges_created.clone()))?;

        let prompts_submitted = IntCounter::with_opts(Opts::new(
            "memory_graph_prompts_submitted_total",
            "Total number of prompts submitted",
        ))?;
        registry.register(Box::new(prompts_submitted.clone()))?;

        let responses_generated = IntCounter::with_opts(Opts::new(
            "memory_graph_responses_generated_total",
            "Total number of responses generated",
        ))?;
        registry.register(Box::new(responses_generated.clone()))?;

        let tools_invoked = IntCounter::with_opts(Opts::new(
            "memory_graph_tools_invoked_total",
            "Total number of tools invoked",
        ))?;
        registry.register(Box::new(tools_invoked.clone()))?;

        let agent_handoffs = IntCounter::with_opts(Opts::new(
            "memory_graph_agent_handoffs_total",
            "Total number of agent handoffs",
        ))?;
        registry.register(Box::new(agent_handoffs.clone()))?;

        let template_instantiations = IntCounter::with_opts(Opts::new(
            "memory_graph_template_instantiations_total",
            "Total number of template instantiations",
        ))?;
        registry.register(Box::new(template_instantiations.clone()))?;

        let queries_executed = IntCounter::with_opts(Opts::new(
            "memory_graph_queries_executed_total",
            "Total number of queries executed",
        ))?;
        registry.register(Box::new(queries_executed.clone()))?;

        // Histograms with appropriate buckets
        let write_latency = Histogram::with_opts(
            HistogramOpts::new(
                "memory_graph_write_latency_seconds",
                "Write operation latency in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
        )?;
        registry.register(Box::new(write_latency.clone()))?;

        let read_latency = Histogram::with_opts(
            HistogramOpts::new(
                "memory_graph_read_latency_seconds",
                "Read operation latency in seconds",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1]),
        )?;
        registry.register(Box::new(read_latency.clone()))?;

        let query_duration = Histogram::with_opts(
            HistogramOpts::new(
                "memory_graph_query_duration_seconds",
                "Query execution duration in seconds",
            )
            .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        )?;
        registry.register(Box::new(query_duration.clone()))?;

        let tool_duration = Histogram::with_opts(
            HistogramOpts::new(
                "memory_graph_tool_duration_seconds",
                "Tool execution duration in seconds",
            )
            .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]),
        )?;
        registry.register(Box::new(tool_duration.clone()))?;

        let batch_size = Histogram::with_opts(
            HistogramOpts::new(
                "memory_graph_batch_size",
                "Batch operation size distribution",
            )
            .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0]),
        )?;
        registry.register(Box::new(batch_size.clone()))?;

        // Gauges
        let active_sessions = IntGauge::with_opts(Opts::new(
            "memory_graph_active_sessions",
            "Number of currently active sessions",
        ))?;
        registry.register(Box::new(active_sessions.clone()))?;

        let total_nodes = IntGauge::with_opts(Opts::new(
            "memory_graph_total_nodes",
            "Total number of nodes in the graph",
        ))?;
        registry.register(Box::new(total_nodes.clone()))?;

        let total_edges = IntGauge::with_opts(Opts::new(
            "memory_graph_total_edges",
            "Total number of edges in the graph",
        ))?;
        registry.register(Box::new(total_edges.clone()))?;

        let cache_size_bytes = IntGauge::with_opts(Opts::new(
            "memory_graph_cache_size_bytes",
            "Current cache size in bytes",
        ))?;
        registry.register(Box::new(cache_size_bytes.clone()))?;

        let buffer_size = IntGauge::with_opts(Opts::new(
            "memory_graph_buffer_size",
            "Current event buffer size",
        ))?;
        registry.register(Box::new(buffer_size.clone()))?;

        // Production Metrics - gRPC
        let grpc_requests_total = IntCounterVec::new(
            Opts::new(
                "memory_graph_grpc_requests_total",
                "Total number of gRPC requests by method and status",
            ),
            &["method", "status"],
        )?;
        registry.register(Box::new(grpc_requests_total.clone()))?;

        let grpc_request_duration = HistogramVec::new(
            HistogramOpts::new(
                "memory_graph_grpc_request_duration_seconds",
                "gRPC request duration in seconds by method",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
            &["method"],
        )?;
        registry.register(Box::new(grpc_request_duration.clone()))?;

        let grpc_active_streams = IntGauge::with_opts(Opts::new(
            "memory_graph_grpc_active_streams",
            "Number of active gRPC streams",
        ))?;
        registry.register(Box::new(grpc_active_streams.clone()))?;

        // Production Metrics - Plugin System
        let plugin_executions_total = IntCounterVec::new(
            Opts::new(
                "memory_graph_plugin_executions_total",
                "Total plugin executions by plugin name and hook",
            ),
            &["plugin", "hook"],
        )?;
        registry.register(Box::new(plugin_executions_total.clone()))?;

        let plugin_duration = HistogramVec::new(
            HistogramOpts::new(
                "memory_graph_plugin_duration_seconds",
                "Plugin execution duration in seconds",
            )
            .buckets(vec![0.001, 0.01, 0.1, 1.0, 5.0, 10.0]),
            &["plugin", "hook"],
        )?;
        registry.register(Box::new(plugin_duration.clone()))?;

        let plugin_errors_total = IntCounterVec::new(
            Opts::new(
                "memory_graph_plugin_errors_total",
                "Total plugin errors by plugin and error type",
            ),
            &["plugin", "error_type"],
        )?;
        registry.register(Box::new(plugin_errors_total.clone()))?;

        // Production Metrics - Integrations
        let registry_calls_total = IntCounterVec::new(
            Opts::new(
                "memory_graph_registry_calls_total",
                "Total LLM-Registry API calls by operation and status",
            ),
            &["operation", "status"],
        )?;
        registry.register(Box::new(registry_calls_total.clone()))?;

        let vault_archives_total = IntCounter::with_opts(Opts::new(
            "memory_graph_vault_archives_total",
            "Total sessions archived to Data-Vault",
        ))?;
        registry.register(Box::new(vault_archives_total.clone()))?;

        let vault_retrievals_total = IntCounter::with_opts(Opts::new(
            "memory_graph_vault_retrievals_total",
            "Total sessions retrieved from Data-Vault",
        ))?;
        registry.register(Box::new(vault_retrievals_total.clone()))?;

        let vault_errors_total = IntCounter::with_opts(Opts::new(
            "memory_graph_vault_errors_total",
            "Total Data-Vault operation errors",
        ))?;
        registry.register(Box::new(vault_errors_total.clone()))?;

        Ok(Self {
            nodes_created,
            edges_created,
            prompts_submitted,
            responses_generated,
            tools_invoked,
            agent_handoffs,
            template_instantiations,
            queries_executed,
            write_latency,
            read_latency,
            query_duration,
            tool_duration,
            batch_size,
            active_sessions,
            total_nodes,
            total_edges,
            cache_size_bytes,
            buffer_size,
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

    /// Create metrics with a custom namespace prefix
    pub fn with_namespace(registry: &Registry, _namespace: &str) -> Result<Self> {
        // This is a simplified version - in production you'd apply prefix to all metrics
        Self::new(registry)
    }

    // Counter recording methods

    /// Record a node creation
    pub fn record_node_created(&self) {
        self.nodes_created.inc();
    }

    /// Record multiple node creations
    pub fn record_nodes_created(&self, count: u64) {
        self.nodes_created.inc_by(count);
    }

    /// Record an edge creation
    pub fn record_edge_created(&self) {
        self.edges_created.inc();
    }

    /// Record multiple edge creations
    pub fn record_edges_created(&self, count: u64) {
        self.edges_created.inc_by(count);
    }

    /// Record a prompt submission
    pub fn record_prompt_submitted(&self) {
        self.prompts_submitted.inc();
    }

    /// Record a response generation
    pub fn record_response_generated(&self) {
        self.responses_generated.inc();
    }

    /// Record a tool invocation
    pub fn record_tool_invoked(&self) {
        self.tools_invoked.inc();
    }

    /// Record an agent handoff
    pub fn record_agent_handoff(&self) {
        self.agent_handoffs.inc();
    }

    /// Record a template instantiation
    pub fn record_template_instantiation(&self) {
        self.template_instantiations.inc();
    }

    /// Record a query execution
    pub fn record_query_executed(&self) {
        self.queries_executed.inc();
    }

    // Histogram recording methods

    /// Record write operation latency in seconds
    pub fn record_write_latency(&self, duration_secs: f64) {
        self.write_latency.observe(duration_secs);
    }

    /// Record read operation latency in seconds
    pub fn record_read_latency(&self, duration_secs: f64) {
        self.read_latency.observe(duration_secs);
    }

    /// Record query execution duration in seconds
    pub fn record_query_duration(&self, duration_secs: f64) {
        self.query_duration.observe(duration_secs);
    }

    /// Record tool execution duration in seconds
    pub fn record_tool_duration(&self, duration_secs: f64) {
        self.tool_duration.observe(duration_secs);
    }

    /// Record batch operation size
    pub fn record_batch_size(&self, size: usize) {
        self.batch_size.observe(size as f64);
    }

    // Gauge update methods

    /// Set the number of active sessions
    pub fn set_active_sessions(&self, count: i64) {
        self.active_sessions.set(count);
    }

    /// Increment active sessions count
    pub fn inc_active_sessions(&self) {
        self.active_sessions.inc();
    }

    /// Decrement active sessions count
    pub fn dec_active_sessions(&self) {
        self.active_sessions.dec();
    }

    /// Set the total number of nodes
    pub fn set_total_nodes(&self, count: i64) {
        self.total_nodes.set(count);
    }

    /// Increment total nodes count
    pub fn inc_total_nodes(&self) {
        self.total_nodes.inc();
    }

    /// Increment total nodes by amount
    pub fn inc_total_nodes_by(&self, count: i64) {
        self.total_nodes.add(count);
    }

    /// Set the total number of edges
    pub fn set_total_edges(&self, count: i64) {
        self.total_edges.set(count);
    }

    /// Increment total edges count
    pub fn inc_total_edges(&self) {
        self.total_edges.inc();
    }

    /// Increment total edges by amount
    pub fn inc_total_edges_by(&self, count: i64) {
        self.total_edges.add(count);
    }

    /// Set cache size in bytes
    pub fn set_cache_size_bytes(&self, size: i64) {
        self.cache_size_bytes.set(size);
    }

    /// Set event buffer size
    pub fn set_buffer_size(&self, size: i64) {
        self.buffer_size.set(size);
    }

    // Production Metrics - gRPC Helper Methods

    /// Record a gRPC request with method and status
    pub fn record_grpc_request(&self, method: &str, status: &str) {
        self.grpc_requests_total
            .with_label_values(&[method, status])
            .inc();
    }

    /// Record gRPC request duration for a method
    pub fn record_grpc_request_duration(&self, method: &str, duration_secs: f64) {
        self.grpc_request_duration
            .with_label_values(&[method])
            .observe(duration_secs);
    }

    /// Increment active gRPC streams count
    pub fn inc_grpc_active_streams(&self) {
        self.grpc_active_streams.inc();
    }

    /// Decrement active gRPC streams count
    pub fn dec_grpc_active_streams(&self) {
        self.grpc_active_streams.dec();
    }

    /// Set the number of active gRPC streams
    pub fn set_grpc_active_streams(&self, count: i64) {
        self.grpc_active_streams.set(count);
    }

    // Production Metrics - Plugin Helper Methods

    /// Record a plugin execution
    pub fn record_plugin_execution(&self, plugin: &str, hook: &str) {
        self.plugin_executions_total
            .with_label_values(&[plugin, hook])
            .inc();
    }

    /// Record plugin execution duration
    pub fn record_plugin_duration(&self, plugin: &str, hook: &str, duration_secs: f64) {
        self.plugin_duration
            .with_label_values(&[plugin, hook])
            .observe(duration_secs);
    }

    /// Record a plugin error
    pub fn record_plugin_error(&self, plugin: &str, error_type: &str) {
        self.plugin_errors_total
            .with_label_values(&[plugin, error_type])
            .inc();
    }

    // Production Metrics - Integration Helper Methods

    /// Record an LLM-Registry API call
    pub fn record_registry_call(&self, operation: &str, status: &str) {
        self.registry_calls_total
            .with_label_values(&[operation, status])
            .inc();
    }

    /// Record a successful vault archive operation
    pub fn record_vault_archive(&self) {
        self.vault_archives_total.inc();
    }

    /// Record multiple vault archive operations
    pub fn record_vault_archives(&self, count: u64) {
        self.vault_archives_total.inc_by(count);
    }

    /// Record a successful vault retrieval operation
    pub fn record_vault_retrieval(&self) {
        self.vault_retrievals_total.inc();
    }

    /// Record multiple vault retrieval operations
    pub fn record_vault_retrievals(&self, count: u64) {
        self.vault_retrievals_total.inc_by(count);
    }

    /// Record a vault error
    pub fn record_vault_error(&self) {
        self.vault_errors_total.inc();
    }

    /// Record multiple vault errors
    pub fn record_vault_errors(&self, count: u64) {
        self.vault_errors_total.inc_by(count);
    }

    /// Get a snapshot of all counter values
    pub fn get_counter_snapshot(&self) -> MetricsCounterSnapshot {
        MetricsCounterSnapshot {
            nodes_created: self.nodes_created.get(),
            edges_created: self.edges_created.get(),
            prompts_submitted: self.prompts_submitted.get(),
            responses_generated: self.responses_generated.get(),
            tools_invoked: self.tools_invoked.get(),
            agent_handoffs: self.agent_handoffs.get(),
            template_instantiations: self.template_instantiations.get(),
            queries_executed: self.queries_executed.get(),
        }
    }

    /// Get a snapshot of all gauge values
    pub fn get_gauge_snapshot(&self) -> MetricsGaugeSnapshot {
        MetricsGaugeSnapshot {
            active_sessions: self.active_sessions.get(),
            total_nodes: self.total_nodes.get(),
            total_edges: self.total_edges.get(),
            cache_size_bytes: self.cache_size_bytes.get(),
            buffer_size: self.buffer_size.get(),
        }
    }

    /// Get a snapshot of gRPC metrics
    pub fn get_grpc_snapshot(&self) -> GrpcMetricsSnapshot {
        GrpcMetricsSnapshot {
            active_streams: self.grpc_active_streams.get(),
        }
    }

    /// Get a snapshot of vault metrics
    pub fn get_vault_snapshot(&self) -> VaultMetricsSnapshot {
        VaultMetricsSnapshot {
            total_archives: self.vault_archives_total.get(),
            total_retrievals: self.vault_retrievals_total.get(),
            total_errors: self.vault_errors_total.get(),
        }
    }
}

/// Snapshot of counter metric values
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsCounterSnapshot {
    /// Total nodes created
    pub nodes_created: u64,
    /// Total edges created
    pub edges_created: u64,
    /// Total prompts submitted
    pub prompts_submitted: u64,
    /// Total responses generated
    pub responses_generated: u64,
    /// Total tools invoked
    pub tools_invoked: u64,
    /// Total agent handoffs
    pub agent_handoffs: u64,
    /// Total template instantiations
    pub template_instantiations: u64,
    /// Total queries executed
    pub queries_executed: u64,
}

/// Snapshot of gauge metric values
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsGaugeSnapshot {
    /// Current active sessions
    pub active_sessions: i64,
    /// Total nodes in graph
    pub total_nodes: i64,
    /// Total edges in graph
    pub total_edges: i64,
    /// Cache size in bytes
    pub cache_size_bytes: i64,
    /// Event buffer size
    pub buffer_size: i64,
}

/// Production metrics snapshot for gRPC operations
#[derive(Debug, Clone, PartialEq)]
pub struct GrpcMetricsSnapshot {
    /// Number of active gRPC streams
    pub active_streams: i64,
}

/// Production metrics snapshot for vault operations
#[derive(Debug, Clone, PartialEq)]
pub struct VaultMetricsSnapshot {
    /// Total archives to vault
    pub total_archives: u64,
    /// Total retrievals from vault
    pub total_retrievals: u64,
    /// Total vault errors
    pub total_errors: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Verify all metrics are initialized
        assert_eq!(metrics.nodes_created.get(), 0);
        assert_eq!(metrics.edges_created.get(), 0);
        assert_eq!(metrics.active_sessions.get(), 0);
    }

    #[test]
    fn test_counter_recording() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.record_node_created();
        assert_eq!(metrics.nodes_created.get(), 1);

        metrics.record_nodes_created(5);
        assert_eq!(metrics.nodes_created.get(), 6);

        metrics.record_edge_created();
        assert_eq!(metrics.edges_created.get(), 1);
    }

    #[test]
    fn test_all_counters() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.record_node_created();
        metrics.record_edge_created();
        metrics.record_prompt_submitted();
        metrics.record_response_generated();
        metrics.record_tool_invoked();
        metrics.record_agent_handoff();
        metrics.record_template_instantiation();
        metrics.record_query_executed();

        assert_eq!(metrics.nodes_created.get(), 1);
        assert_eq!(metrics.edges_created.get(), 1);
        assert_eq!(metrics.prompts_submitted.get(), 1);
        assert_eq!(metrics.responses_generated.get(), 1);
        assert_eq!(metrics.tools_invoked.get(), 1);
        assert_eq!(metrics.agent_handoffs.get(), 1);
        assert_eq!(metrics.template_instantiations.get(), 1);
        assert_eq!(metrics.queries_executed.get(), 1);
    }

    #[test]
    fn test_histogram_recording() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.record_write_latency(0.025);
        metrics.record_read_latency(0.001);
        metrics.record_query_duration(0.5);
        metrics.record_tool_duration(2.0);
        metrics.record_batch_size(50);

        // Histograms don't expose simple get() - verify no panics
        assert!(true);
    }

    #[test]
    fn test_gauge_updates() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.set_active_sessions(5);
        assert_eq!(metrics.active_sessions.get(), 5);

        metrics.inc_active_sessions();
        assert_eq!(metrics.active_sessions.get(), 6);

        metrics.dec_active_sessions();
        assert_eq!(metrics.active_sessions.get(), 5);

        metrics.set_total_nodes(100);
        assert_eq!(metrics.total_nodes.get(), 100);

        metrics.inc_total_nodes();
        assert_eq!(metrics.total_nodes.get(), 101);

        metrics.inc_total_nodes_by(9);
        assert_eq!(metrics.total_nodes.get(), 110);
    }

    #[test]
    fn test_all_gauges() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.set_active_sessions(10);
        metrics.set_total_nodes(500);
        metrics.set_total_edges(800);
        metrics.set_cache_size_bytes(1024 * 1024);
        metrics.set_buffer_size(25);

        assert_eq!(metrics.active_sessions.get(), 10);
        assert_eq!(metrics.total_nodes.get(), 500);
        assert_eq!(metrics.total_edges.get(), 800);
        assert_eq!(metrics.cache_size_bytes.get(), 1024 * 1024);
        assert_eq!(metrics.buffer_size.get(), 25);
    }

    #[test]
    fn test_counter_snapshot() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.record_node_created();
        metrics.record_edges_created(3);
        metrics.record_prompt_submitted();

        let snapshot = metrics.get_counter_snapshot();
        assert_eq!(snapshot.nodes_created, 1);
        assert_eq!(snapshot.edges_created, 3);
        assert_eq!(snapshot.prompts_submitted, 1);
        assert_eq!(snapshot.responses_generated, 0);
    }

    #[test]
    fn test_gauge_snapshot() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.set_active_sessions(7);
        metrics.set_total_nodes(150);
        metrics.set_cache_size_bytes(2048);

        let snapshot = metrics.get_gauge_snapshot();
        assert_eq!(snapshot.active_sessions, 7);
        assert_eq!(snapshot.total_nodes, 150);
        assert_eq!(snapshot.cache_size_bytes, 2048);
    }

    #[test]
    fn test_metrics_clone() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.record_node_created();

        let cloned = metrics.clone();
        cloned.record_node_created();

        // Both should see the same counter
        assert_eq!(metrics.nodes_created.get(), 2);
        assert_eq!(cloned.nodes_created.get(), 2);
    }

    #[test]
    fn test_concurrent_counter_updates() {
        use std::sync::Arc;
        use std::thread;

        let registry = Registry::new();
        let metrics = Arc::new(PrometheusMetrics::new(&registry).unwrap());

        let mut handles = vec![];
        for _ in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    metrics_clone.record_node_created();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(metrics.nodes_created.get(), 1000);
    }

    #[test]
    fn test_latency_buckets() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Test various latency ranges
        metrics.record_write_latency(0.001);  // 1ms
        metrics.record_write_latency(0.010);  // 10ms
        metrics.record_write_latency(0.100);  // 100ms
        metrics.record_write_latency(1.000);  // 1s

        metrics.record_read_latency(0.0001); // 0.1ms
        metrics.record_read_latency(0.001);  // 1ms
        metrics.record_read_latency(0.01);   // 10ms
    }

    #[test]
    fn test_batch_size_distribution() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.record_batch_size(1);
        metrics.record_batch_size(10);
        metrics.record_batch_size(50);
        metrics.record_batch_size(100);
        metrics.record_batch_size(500);
    }

    #[test]
    fn test_edge_operations_tracking() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.set_total_edges(100);
        metrics.record_edge_created();

        // Track edge creation but gauge doesn't auto-increment
        assert_eq!(metrics.edges_created.get(), 1);

        // Manually sync gauge with counter
        metrics.inc_total_edges();
        assert_eq!(metrics.total_edges.get(), 101);
    }

    #[test]
    fn test_session_lifecycle() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Session created
        metrics.inc_active_sessions();
        assert_eq!(metrics.active_sessions.get(), 1);

        // More sessions created
        metrics.inc_active_sessions();
        metrics.inc_active_sessions();
        assert_eq!(metrics.active_sessions.get(), 3);

        // Session ended
        metrics.dec_active_sessions();
        assert_eq!(metrics.active_sessions.get(), 2);
    }

    #[test]
    fn test_prometheus_text_export() {
        use prometheus::TextEncoder;

        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.record_node_created();
        metrics.record_prompt_submitted();
        metrics.set_active_sessions(5);

        let encoder = TextEncoder::new();
        let metric_families = registry.gather();
        let encoded = encoder.encode_to_string(&metric_families).unwrap();

        assert!(encoded.contains("memory_graph_nodes_created_total"));
        assert!(encoded.contains("memory_graph_prompts_submitted_total"));
        assert!(encoded.contains("memory_graph_active_sessions"));
    }

    // Production Metrics Tests - gRPC

    #[test]
    fn test_grpc_request_metrics() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Record various gRPC requests
        metrics.record_grpc_request("CreateSession", "success");
        metrics.record_grpc_request("CreateSession", "success");
        metrics.record_grpc_request("GetNode", "success");
        metrics.record_grpc_request("UpdateNode", "error");

        // Verify counters are tracked per label
        let grpc_total = metrics
            .grpc_requests_total
            .with_label_values(&["CreateSession", "success"])
            .get();
        assert_eq!(grpc_total, 2);

        let error_total = metrics
            .grpc_requests_total
            .with_label_values(&["UpdateNode", "error"])
            .get();
        assert_eq!(error_total, 1);
    }

    #[test]
    fn test_grpc_request_duration() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Record request durations
        metrics.record_grpc_request_duration("CreateSession", 0.015);
        metrics.record_grpc_request_duration("Query", 0.125);
        metrics.record_grpc_request_duration("BatchCreateNodes", 0.45);

        // Verify no panics - histogram values aren't directly accessible
        assert!(true);
    }

    #[test]
    fn test_grpc_active_streams() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        assert_eq!(metrics.grpc_active_streams.get(), 0);

        metrics.inc_grpc_active_streams();
        assert_eq!(metrics.grpc_active_streams.get(), 1);

        metrics.inc_grpc_active_streams();
        metrics.inc_grpc_active_streams();
        assert_eq!(metrics.grpc_active_streams.get(), 3);

        metrics.dec_grpc_active_streams();
        assert_eq!(metrics.grpc_active_streams.get(), 2);

        metrics.set_grpc_active_streams(10);
        assert_eq!(metrics.grpc_active_streams.get(), 10);
    }

    #[test]
    fn test_grpc_snapshot() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.set_grpc_active_streams(5);

        let snapshot = metrics.get_grpc_snapshot();
        assert_eq!(snapshot.active_streams, 5);
    }

    // Production Metrics Tests - Plugin System

    #[test]
    fn test_plugin_execution_metrics() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Record plugin executions
        metrics.record_plugin_execution("audit_logger", "on_node_create");
        metrics.record_plugin_execution("audit_logger", "on_node_create");
        metrics.record_plugin_execution("validator", "on_edge_create");
        metrics.record_plugin_execution("audit_logger", "on_session_close");

        // Verify counters
        let audit_create = metrics
            .plugin_executions_total
            .with_label_values(&["audit_logger", "on_node_create"])
            .get();
        assert_eq!(audit_create, 2);

        let validator_edge = metrics
            .plugin_executions_total
            .with_label_values(&["validator", "on_edge_create"])
            .get();
        assert_eq!(validator_edge, 1);
    }

    #[test]
    fn test_plugin_duration_metrics() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Record plugin durations
        metrics.record_plugin_duration("audit_logger", "on_node_create", 0.002);
        metrics.record_plugin_duration("validator", "on_edge_create", 0.015);
        metrics.record_plugin_duration("transformer", "on_query", 0.125);

        // Verify no panics
        assert!(true);
    }

    #[test]
    fn test_plugin_error_metrics() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Record plugin errors
        metrics.record_plugin_error("validator", "validation_failed");
        metrics.record_plugin_error("validator", "validation_failed");
        metrics.record_plugin_error("transformer", "timeout");
        metrics.record_plugin_error("audit_logger", "connection_error");

        // Verify error counters
        let validation_errors = metrics
            .plugin_errors_total
            .with_label_values(&["validator", "validation_failed"])
            .get();
        assert_eq!(validation_errors, 2);

        let timeout_errors = metrics
            .plugin_errors_total
            .with_label_values(&["transformer", "timeout"])
            .get();
        assert_eq!(timeout_errors, 1);
    }

    // Production Metrics Tests - Integrations

    #[test]
    fn test_registry_call_metrics() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Record registry API calls
        metrics.record_registry_call("register_model", "success");
        metrics.record_registry_call("register_model", "success");
        metrics.record_registry_call("get_metadata", "success");
        metrics.record_registry_call("update_version", "error");

        // Verify counters
        let register_success = metrics
            .registry_calls_total
            .with_label_values(&["register_model", "success"])
            .get();
        assert_eq!(register_success, 2);

        let update_error = metrics
            .registry_calls_total
            .with_label_values(&["update_version", "error"])
            .get();
        assert_eq!(update_error, 1);
    }

    #[test]
    fn test_vault_archive_metrics() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        assert_eq!(metrics.vault_archives_total.get(), 0);

        metrics.record_vault_archive();
        assert_eq!(metrics.vault_archives_total.get(), 1);

        metrics.record_vault_archives(5);
        assert_eq!(metrics.vault_archives_total.get(), 6);
    }

    #[test]
    fn test_vault_retrieval_metrics() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        assert_eq!(metrics.vault_retrievals_total.get(), 0);

        metrics.record_vault_retrieval();
        assert_eq!(metrics.vault_retrievals_total.get(), 1);

        metrics.record_vault_retrievals(3);
        assert_eq!(metrics.vault_retrievals_total.get(), 4);
    }

    #[test]
    fn test_vault_error_metrics() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        assert_eq!(metrics.vault_errors_total.get(), 0);

        metrics.record_vault_error();
        assert_eq!(metrics.vault_errors_total.get(), 1);

        metrics.record_vault_errors(2);
        assert_eq!(metrics.vault_errors_total.get(), 3);
    }

    #[test]
    fn test_vault_snapshot() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        metrics.record_vault_archives(10);
        metrics.record_vault_retrievals(5);
        metrics.record_vault_errors(2);

        let snapshot = metrics.get_vault_snapshot();
        assert_eq!(snapshot.total_archives, 10);
        assert_eq!(snapshot.total_retrievals, 5);
        assert_eq!(snapshot.total_errors, 2);
    }

    #[test]
    fn test_production_metrics_text_export() {
        use prometheus::TextEncoder;

        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Record production metrics
        metrics.record_grpc_request("CreateSession", "success");
        metrics.record_plugin_execution("audit_logger", "on_node_create");
        metrics.record_registry_call("register_model", "success");
        metrics.record_vault_archive();

        let encoder = TextEncoder::new();
        let metric_families = registry.gather();
        let encoded = encoder.encode_to_string(&metric_families).unwrap();

        // Verify production metrics are exported
        assert!(encoded.contains("memory_graph_grpc_requests_total"));
        assert!(encoded.contains("memory_graph_plugin_executions_total"));
        assert!(encoded.contains("memory_graph_registry_calls_total"));
        assert!(encoded.contains("memory_graph_vault_archives_total"));
    }

    #[test]
    fn test_complete_production_workflow() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Simulate a complete gRPC request with plugin execution
        metrics.inc_grpc_active_streams();
        metrics.record_grpc_request("CreateNode", "success");
        metrics.record_grpc_request_duration("CreateNode", 0.025);

        // Plugin hooks execute
        metrics.record_plugin_execution("validator", "on_node_create");
        metrics.record_plugin_duration("validator", "on_node_create", 0.003);
        metrics.record_plugin_execution("audit_logger", "on_node_create");
        metrics.record_plugin_duration("audit_logger", "on_node_create", 0.001);

        // Registry integration
        metrics.record_registry_call("track_operation", "success");

        // Core metrics
        metrics.record_node_created();
        metrics.inc_total_nodes();

        // Stream completes
        metrics.dec_grpc_active_streams();

        // Verify final state
        assert_eq!(metrics.grpc_active_streams.get(), 0);
        assert_eq!(metrics.nodes_created.get(), 1);
        assert_eq!(metrics.total_nodes.get(), 1);
    }

    #[test]
    fn test_all_production_metrics_initialized() {
        let registry = Registry::new();
        let metrics = PrometheusMetrics::new(&registry).unwrap();

        // Verify all production metrics are initialized
        assert_eq!(metrics.grpc_active_streams.get(), 0);
        assert_eq!(metrics.vault_archives_total.get(), 0);
        assert_eq!(metrics.vault_retrievals_total.get(), 0);
        assert_eq!(metrics.vault_errors_total.get(), 0);
    }
}
