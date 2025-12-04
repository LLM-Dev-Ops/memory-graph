//! Ingestion pipeline for processing telemetry data
//!
//! This module provides a unified pipeline for ingesting, processing, and transforming
//! telemetry data from various sources into memory graph structures.

use super::consumer::{ConsumptionStats, TelemetryConsumer, TelemetryData};
use super::lineage::{LineageBuilder, LineageChain};
use super::mapping::{MappingConfig, MappingResult, SpanMapper};
use super::temporal::{TemporalGraph, TemporalGraphBuilder};
use crate::Result;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for the ingestion pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionConfig {
    /// Enable lineage building from spans
    pub enable_lineage: bool,
    /// Enable temporal graph building from metrics
    pub enable_temporal: bool,
    /// Enable span-to-entity mapping
    pub enable_mapping: bool,
    /// Mapping configuration
    pub mapping_config: MappingConfig,
    /// Buffer size for batching
    pub buffer_size: usize,
    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
    /// Retention period for temporal data
    pub temporal_retention_hours: i64,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            enable_lineage: true,
            enable_temporal: true,
            enable_mapping: true,
            mapping_config: MappingConfig::default(),
            buffer_size: 100,
            flush_interval_ms: 1000,
            temporal_retention_hours: 24,
        }
    }
}

/// Statistics about the ingestion pipeline
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestionStats {
    /// Total items ingested
    pub total_ingested: u64,
    /// Spans processed
    pub spans_processed: u64,
    /// Metrics processed
    pub metrics_processed: u64,
    /// Logs processed
    pub logs_processed: u64,
    /// Lineage chains built
    pub lineage_chains_built: u64,
    /// Entities mapped
    pub entities_mapped: u64,
    /// Mapping errors
    pub mapping_errors: u64,
    /// Last ingestion timestamp
    pub last_ingestion: Option<DateTime<Utc>>,
}

/// Result of processing telemetry through the pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    /// Whether processing was successful
    pub success: bool,
    /// Lineage chain (if built)
    pub lineage_chain: Option<LineageChain>,
    /// Mapping result (if enabled)
    pub mapping_result: Option<MappingResult>,
    /// Processing errors
    pub errors: Vec<String>,
}

impl ProcessingResult {
    /// Create a successful result
    pub fn success() -> Self {
        Self {
            success: true,
            lineage_chain: None,
            mapping_result: None,
            errors: Vec::new(),
        }
    }

    /// Create a failed result with error
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            lineage_chain: None,
            mapping_result: None,
            errors: vec![message],
        }
    }

    /// Add an error to the result
    pub fn add_error(&mut self, error: String) {
        self.success = false;
        self.errors.push(error);
    }
}

/// Ingestion pipeline for processing telemetry data
pub struct IngestionPipeline {
    /// Configuration
    config: IngestionConfig,
    /// Lineage builder
    lineage_builder: Arc<LineageBuilder>,
    /// Temporal graph builder
    temporal_builder: Arc<TemporalGraphBuilder>,
    /// Span mapper
    mapper: Arc<RwLock<SpanMapper>>,
    /// Pipeline statistics
    stats: Arc<RwLock<IngestionStats>>,
    /// Buffer for batching
    buffer: Arc<RwLock<Vec<TelemetryData>>>,
}

impl IngestionPipeline {
    /// Create a new ingestion pipeline with default configuration
    pub fn new() -> Self {
        Self::with_config(IngestionConfig::default())
    }

    /// Create a new ingestion pipeline with custom configuration
    pub fn with_config(config: IngestionConfig) -> Self {
        let retention = Duration::hours(config.temporal_retention_hours);
        let mapper = SpanMapper::with_config(config.mapping_config.clone());

        Self {
            config,
            lineage_builder: Arc::new(LineageBuilder::new()),
            temporal_builder: Arc::new(TemporalGraphBuilder::with_retention(retention)),
            mapper: Arc::new(RwLock::new(mapper)),
            stats: Arc::new(RwLock::new(IngestionStats::default())),
            buffer: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Ingest a single telemetry data point
    pub async fn ingest(&self, data: TelemetryData) -> Result<ProcessingResult> {
        let mut result = ProcessingResult::success();

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_ingested += 1;
        match &data {
            TelemetryData::Span { .. } => stats.spans_processed += 1,
            TelemetryData::Metric { .. } => stats.metrics_processed += 1,
            TelemetryData::Log { .. } => stats.logs_processed += 1,
        }
        stats.last_ingestion = Some(Utc::now());
        drop(stats);

        // Process based on data type
        match &data {
            TelemetryData::Span { .. } => {
                if let Err(e) = self.process_span(&data, &mut result).await {
                    result.add_error(format!("Span processing error: {}", e));
                }
            }
            TelemetryData::Metric { .. } => {
                if let Err(e) = self.process_metric(&data).await {
                    result.add_error(format!("Metric processing error: {}", e));
                }
            }
            TelemetryData::Log { .. } => {
                if let Err(e) = self.process_log(&data).await {
                    result.add_error(format!("Log processing error: {}", e));
                }
            }
        }

        Ok(result)
    }

    /// Ingest a batch of telemetry data
    pub async fn ingest_batch(&self, data: Vec<TelemetryData>) -> Result<Vec<ProcessingResult>> {
        let mut results = Vec::new();
        for item in data {
            results.push(self.ingest(item).await?);
        }
        Ok(results)
    }

    /// Process a span
    async fn process_span(
        &self,
        data: &TelemetryData,
        result: &mut ProcessingResult,
    ) -> Result<()> {
        // Build lineage if enabled
        if self.config.enable_lineage {
            self.lineage_builder.process_span(data).await?;
        }

        // Map to graph entities if enabled
        if self.config.enable_mapping {
            let mut mapper = self.mapper.write().await;
            if let Some(entity) = mapper.map_span(data) {
                let mut stats = self.stats.write().await;
                stats.entities_mapped += 1;
                drop(stats);

                let mut mapping_result = MappingResult::new();
                mapping_result.add_entity(entity);
                result.mapping_result = Some(mapping_result);
            } else {
                let mut stats = self.stats.write().await;
                stats.mapping_errors += 1;
                drop(stats);
                result.add_error("Failed to map span to entity".to_string());
            }
        }

        Ok(())
    }

    /// Process a metric
    async fn process_metric(&self, data: &TelemetryData) -> Result<()> {
        if self.config.enable_temporal {
            self.temporal_builder.process_metric(data).await?;
        }
        Ok(())
    }

    /// Process a log
    async fn process_log(&self, _data: &TelemetryData) -> Result<()> {
        // Log processing can be extended in the future
        // For now, we just acknowledge it
        Ok(())
    }

    /// Get a lineage chain by trace ID
    pub async fn get_lineage_chain(&self, trace_id: &str) -> Option<LineageChain> {
        self.lineage_builder.get_chain(trace_id).await
    }

    /// Build a temporal graph for a time range
    pub async fn build_temporal_graph(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<TemporalGraph> {
        self.temporal_builder.build_graph(start, end).await
    }

    /// Map a complete lineage chain to graph entities
    pub async fn map_lineage_chain(&self, chain: &LineageChain) -> MappingResult {
        let mut mapper = self.mapper.write().await;
        let result = mapper.map_lineage_chain(chain);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.entities_mapped += result.entities.len() as u64;
        stats.mapping_errors += result.errors.len() as u64;
        if !result.errors.is_empty() {
            stats.lineage_chains_built += 1;
        }

        result
    }

    /// Get pipeline statistics
    pub async fn stats(&self) -> IngestionStats {
        self.stats.read().await.clone()
    }

    /// Reset pipeline statistics
    pub async fn reset_stats(&self) {
        *self.stats.write().await = IngestionStats::default();
    }

    /// Clear all pipeline data
    pub async fn clear(&self) {
        self.lineage_builder.clear().await;
        self.temporal_builder.clear().await;
        self.mapper.write().await.clear_cache();
        self.buffer.write().await.clear();
        self.reset_stats().await;
    }

    /// Get the current buffer size
    pub async fn buffer_size(&self) -> usize {
        self.buffer.read().await.len()
    }

    /// Flush the buffer (process all buffered items)
    pub async fn flush(&self) -> Result<Vec<ProcessingResult>> {
        let mut buffer = self.buffer.write().await;
        let items: Vec<_> = buffer.drain(..).collect();
        drop(buffer);

        self.ingest_batch(items).await
    }
}

impl Default for IngestionPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TelemetryConsumer for IngestionPipeline {
    async fn consume(&self, data: TelemetryData) -> Result<()> {
        self.ingest(data).await?;
        Ok(())
    }

    async fn consume_batch(&self, data: Vec<TelemetryData>) -> Result<()> {
        self.ingest_batch(data).await?;
        Ok(())
    }

    async fn stats(&self) -> Result<ConsumptionStats> {
        let pipeline_stats = self.stats.read().await;
        Ok(ConsumptionStats {
            spans_consumed: pipeline_stats.spans_processed,
            metrics_consumed: pipeline_stats.metrics_processed,
            logs_consumed: pipeline_stats.logs_processed,
            errors: pipeline_stats.mapping_errors,
            last_consumption: pipeline_stats.last_ingestion,
        })
    }

    async fn reset_stats(&self) -> Result<()> {
        self.reset_stats().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::consumer::SpanStatus;
    use std::collections::HashMap;

    fn create_test_span(span_id: &str, trace_id: &str) -> TelemetryData {
        TelemetryData::Span {
            span_id: span_id.to_string(),
            trace_id: trace_id.to_string(),
            parent_span_id: None,
            operation_name: "llm.generate".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            attributes: HashMap::new(),
            status: SpanStatus::Ok,
        }
    }

    fn create_test_metric(name: &str, value: f64) -> TelemetryData {
        TelemetryData::Metric {
            name: name.to_string(),
            value,
            metric_type: super::super::consumer::MetricType::Gauge,
            timestamp: Utc::now(),
            labels: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_ingestion_pipeline_ingest_span() {
        let pipeline = IngestionPipeline::new();

        let span = create_test_span("span-1", "trace-1");
        let result = pipeline.ingest(span).await.unwrap();

        assert!(result.success);
        assert!(result.mapping_result.is_some());

        let stats = pipeline.stats().await;
        assert_eq!(stats.total_ingested, 1);
        assert_eq!(stats.spans_processed, 1);
    }

    #[tokio::test]
    async fn test_ingestion_pipeline_ingest_metric() {
        let pipeline = IngestionPipeline::new();

        let metric = create_test_metric("cpu.usage", 0.75);
        let result = pipeline.ingest(metric).await.unwrap();

        assert!(result.success);

        let stats = pipeline.stats().await;
        assert_eq!(stats.total_ingested, 1);
        assert_eq!(stats.metrics_processed, 1);
    }

    #[tokio::test]
    async fn test_ingestion_pipeline_batch() {
        let pipeline = IngestionPipeline::new();

        let batch = vec![
            create_test_span("span-1", "trace-1"),
            create_test_span("span-2", "trace-1"),
            create_test_metric("metric-1", 1.0),
        ];

        let results = pipeline.ingest_batch(batch).await.unwrap();
        assert_eq!(results.len(), 3);

        let stats = pipeline.stats().await;
        assert_eq!(stats.total_ingested, 3);
        assert_eq!(stats.spans_processed, 2);
        assert_eq!(stats.metrics_processed, 1);
    }

    #[tokio::test]
    async fn test_ingestion_pipeline_lineage_chain() {
        let pipeline = IngestionPipeline::new();

        let span = create_test_span("span-1", "trace-1");
        pipeline.ingest(span).await.unwrap();

        let chain = pipeline.get_lineage_chain("trace-1").await;
        assert!(chain.is_some());

        let chain = chain.unwrap();
        assert_eq!(chain.nodes.len(), 1);
        assert_eq!(chain.trace_id, "trace-1");
    }

    #[tokio::test]
    async fn test_ingestion_pipeline_temporal_graph() {
        let pipeline = IngestionPipeline::new();

        let now = Utc::now();
        for i in 0..5 {
            let metric = create_test_metric("test.metric", i as f64);
            pipeline.ingest(metric).await.unwrap();
        }

        let graph = pipeline
            .build_temporal_graph(now - Duration::minutes(1), now + Duration::minutes(1))
            .await
            .unwrap();

        assert!(graph.nodes.len() > 0);
    }

    #[tokio::test]
    async fn test_ingestion_pipeline_clear() {
        let pipeline = IngestionPipeline::new();

        let span = create_test_span("span-1", "trace-1");
        pipeline.ingest(span).await.unwrap();

        let stats = pipeline.stats().await;
        assert_eq!(stats.total_ingested, 1);

        pipeline.clear().await;

        let stats = pipeline.stats().await;
        assert_eq!(stats.total_ingested, 0);
    }

    #[tokio::test]
    async fn test_ingestion_config_custom() {
        let config = IngestionConfig {
            enable_lineage: false,
            enable_temporal: false,
            enable_mapping: true,
            mapping_config: MappingConfig::default(),
            buffer_size: 50,
            flush_interval_ms: 500,
            temporal_retention_hours: 12,
        };

        let pipeline = IngestionPipeline::with_config(config);

        let span = create_test_span("span-1", "trace-1");
        pipeline.ingest(span).await.unwrap();

        // Lineage should not be built since it's disabled
        let chain = pipeline.get_lineage_chain("trace-1").await;
        assert!(chain.is_none());
    }

    #[tokio::test]
    async fn test_ingestion_pipeline_as_consumer() {
        let pipeline = IngestionPipeline::new();
        let consumer: &dyn TelemetryConsumer = &pipeline;

        let span = create_test_span("span-1", "trace-1");
        consumer.consume(span).await.unwrap();

        let stats = consumer.stats().await.unwrap();
        assert_eq!(stats.spans_consumed, 1);
    }

    #[tokio::test]
    async fn test_processing_result() {
        let mut result = ProcessingResult::success();
        assert!(result.success);
        assert_eq!(result.errors.len(), 0);

        result.add_error("Test error".to_string());
        assert!(!result.success);
        assert_eq!(result.errors.len(), 1);

        let error_result = ProcessingResult::error("Failed".to_string());
        assert!(!error_result.success);
        assert_eq!(error_result.errors.len(), 1);
    }

    #[tokio::test]
    async fn test_ingestion_stats_tracking() {
        let pipeline = IngestionPipeline::new();

        // Ingest various types
        pipeline.ingest(create_test_span("s1", "t1")).await.unwrap();
        pipeline.ingest(create_test_metric("m1", 1.0)).await.unwrap();

        let stats = pipeline.stats().await;
        assert_eq!(stats.total_ingested, 2);
        assert_eq!(stats.spans_processed, 1);
        assert_eq!(stats.metrics_processed, 1);
        assert!(stats.last_ingestion.is_some());

        pipeline.reset_stats().await;
        let stats = pipeline.stats().await;
        assert_eq!(stats.total_ingested, 0);
    }
}
