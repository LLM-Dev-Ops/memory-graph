//! Telemetry consumer traits and implementations
//!
//! This module provides the consumption side of the bidirectional Observatory integration,
//! allowing the memory graph to ingest telemetry data from external systems and build
//! lineage chains, temporal graphs, and correlation structures.

use crate::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Telemetry data types that can be consumed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "telemetry_type", rename_all = "snake_case")]
pub enum TelemetryData {
    /// Distributed trace span
    Span {
        /// Unique span identifier
        span_id: String,
        /// Trace ID this span belongs to
        trace_id: String,
        /// Parent span ID (if any)
        parent_span_id: Option<String>,
        /// Operation name
        operation_name: String,
        /// Start timestamp
        start_time: DateTime<Utc>,
        /// End timestamp
        end_time: DateTime<Utc>,
        /// Span attributes/tags
        attributes: HashMap<String, String>,
        /// Span status (ok, error, etc.)
        status: SpanStatus,
    },

    /// Metric data point
    Metric {
        /// Metric name
        name: String,
        /// Metric value
        value: f64,
        /// Metric type (counter, gauge, histogram)
        metric_type: MetricType,
        /// Timestamp
        timestamp: DateTime<Utc>,
        /// Metric labels/dimensions
        labels: HashMap<String, String>,
    },

    /// Structured log entry
    Log {
        /// Log level
        level: LogLevel,
        /// Log message
        message: String,
        /// Timestamp
        timestamp: DateTime<Utc>,
        /// Structured fields
        fields: HashMap<String, String>,
        /// Trace context (if available)
        trace_context: Option<TraceContext>,
    },
}

/// Span execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpanStatus {
    /// Span completed successfully
    Ok,
    /// Span encountered an error
    Error,
    /// Span status unknown
    Unset,
}

/// Metric type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricType {
    /// Cumulative counter
    Counter,
    /// Point-in-time gauge
    Gauge,
    /// Distribution histogram
    Histogram,
    /// Summary statistics
    Summary,
}

/// Log severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warning level
    Warn,
    /// Error level
    Error,
    /// Fatal/Critical level
    Fatal,
}

/// Trace context for correlating logs with traces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// Trace ID
    pub trace_id: String,
    /// Span ID
    pub span_id: String,
    /// Trace flags
    pub trace_flags: u8,
}

/// Statistics about consumed telemetry
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsumptionStats {
    /// Total spans consumed
    pub spans_consumed: u64,
    /// Total metrics consumed
    pub metrics_consumed: u64,
    /// Total logs consumed
    pub logs_consumed: u64,
    /// Total consumption errors
    pub errors: u64,
    /// Last consumption timestamp
    pub last_consumption: Option<DateTime<Utc>>,
}

/// Trait for consuming telemetry data
#[async_trait]
pub trait TelemetryConsumer: Send + Sync {
    /// Consume a single telemetry data point
    async fn consume(&self, data: TelemetryData) -> Result<()>;

    /// Consume a batch of telemetry data
    async fn consume_batch(&self, data: Vec<TelemetryData>) -> Result<()> {
        for item in data {
            self.consume(item).await?;
        }
        Ok(())
    }

    /// Get consumption statistics
    async fn stats(&self) -> Result<ConsumptionStats>;

    /// Reset consumption statistics
    async fn reset_stats(&self) -> Result<()>;
}

/// In-memory telemetry consumer for testing and development
#[derive(Clone)]
pub struct InMemoryConsumer {
    /// Consumed telemetry data
    data: Arc<RwLock<Vec<TelemetryData>>>,
    /// Consumption statistics
    stats: Arc<RwLock<ConsumptionStats>>,
}

impl InMemoryConsumer {
    /// Create a new in-memory consumer
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(ConsumptionStats::default())),
        }
    }

    /// Get all consumed telemetry data
    pub async fn get_data(&self) -> Vec<TelemetryData> {
        self.data.read().await.clone()
    }

    /// Get consumed spans only
    pub async fn get_spans(&self) -> Vec<TelemetryData> {
        self.data
            .read()
            .await
            .iter()
            .filter(|d| matches!(d, TelemetryData::Span { .. }))
            .cloned()
            .collect()
    }

    /// Get consumed metrics only
    pub async fn get_metrics(&self) -> Vec<TelemetryData> {
        self.data
            .read()
            .await
            .iter()
            .filter(|d| matches!(d, TelemetryData::Metric { .. }))
            .cloned()
            .collect()
    }

    /// Get consumed logs only
    pub async fn get_logs(&self) -> Vec<TelemetryData> {
        self.data
            .read()
            .await
            .iter()
            .filter(|d| matches!(d, TelemetryData::Log { .. }))
            .cloned()
            .collect()
    }

    /// Clear all consumed data
    pub async fn clear(&self) {
        self.data.write().await.clear();
    }

    /// Get count of consumed items
    pub async fn count(&self) -> usize {
        self.data.read().await.len()
    }
}

impl Default for InMemoryConsumer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TelemetryConsumer for InMemoryConsumer {
    async fn consume(&self, data: TelemetryData) -> Result<()> {
        // Update statistics
        let mut stats = self.stats.write().await;
        match &data {
            TelemetryData::Span { .. } => stats.spans_consumed += 1,
            TelemetryData::Metric { .. } => stats.metrics_consumed += 1,
            TelemetryData::Log { .. } => stats.logs_consumed += 1,
        }
        stats.last_consumption = Some(Utc::now());
        drop(stats);

        // Store data
        self.data.write().await.push(data);
        Ok(())
    }

    async fn consume_batch(&self, data: Vec<TelemetryData>) -> Result<()> {
        // Update statistics in batch
        let mut stats = self.stats.write().await;
        for item in &data {
            match item {
                TelemetryData::Span { .. } => stats.spans_consumed += 1,
                TelemetryData::Metric { .. } => stats.metrics_consumed += 1,
                TelemetryData::Log { .. } => stats.logs_consumed += 1,
            }
        }
        stats.last_consumption = Some(Utc::now());
        drop(stats);

        // Store data
        self.data.write().await.extend(data);
        Ok(())
    }

    async fn stats(&self) -> Result<ConsumptionStats> {
        Ok(self.stats.read().await.clone())
    }

    async fn reset_stats(&self) -> Result<()> {
        *self.stats.write().await = ConsumptionStats::default();
        Ok(())
    }
}

/// No-op consumer that discards all telemetry data
#[derive(Clone, Copy)]
pub struct NoOpConsumer;

#[async_trait]
impl TelemetryConsumer for NoOpConsumer {
    async fn consume(&self, _data: TelemetryData) -> Result<()> {
        Ok(())
    }

    async fn consume_batch(&self, _data: Vec<TelemetryData>) -> Result<()> {
        Ok(())
    }

    async fn stats(&self) -> Result<ConsumptionStats> {
        Ok(ConsumptionStats::default())
    }

    async fn reset_stats(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_consumer_span() {
        let consumer = InMemoryConsumer::new();

        let span = TelemetryData::Span {
            span_id: "span-123".to_string(),
            trace_id: "trace-456".to_string(),
            parent_span_id: None,
            operation_name: "llm.generate".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            attributes: HashMap::new(),
            status: SpanStatus::Ok,
        };

        consumer.consume(span).await.unwrap();

        assert_eq!(consumer.count().await, 1);
        assert_eq!(consumer.get_spans().await.len(), 1);

        let stats = consumer.stats().await.unwrap();
        assert_eq!(stats.spans_consumed, 1);
        assert_eq!(stats.metrics_consumed, 0);
        assert_eq!(stats.logs_consumed, 0);
    }

    #[tokio::test]
    async fn test_in_memory_consumer_metric() {
        let consumer = InMemoryConsumer::new();

        let metric = TelemetryData::Metric {
            name: "llm.tokens.total".to_string(),
            value: 1500.0,
            metric_type: MetricType::Counter,
            timestamp: Utc::now(),
            labels: HashMap::new(),
        };

        consumer.consume(metric).await.unwrap();

        assert_eq!(consumer.count().await, 1);
        assert_eq!(consumer.get_metrics().await.len(), 1);

        let stats = consumer.stats().await.unwrap();
        assert_eq!(stats.metrics_consumed, 1);
    }

    #[tokio::test]
    async fn test_in_memory_consumer_log() {
        let consumer = InMemoryConsumer::new();

        let log = TelemetryData::Log {
            level: LogLevel::Info,
            message: "LLM request completed".to_string(),
            timestamp: Utc::now(),
            fields: HashMap::new(),
            trace_context: None,
        };

        consumer.consume(log).await.unwrap();

        assert_eq!(consumer.count().await, 1);
        assert_eq!(consumer.get_logs().await.len(), 1);

        let stats = consumer.stats().await.unwrap();
        assert_eq!(stats.logs_consumed, 1);
    }

    #[tokio::test]
    async fn test_in_memory_consumer_batch() {
        let consumer = InMemoryConsumer::new();

        let data = vec![
            TelemetryData::Span {
                span_id: "span-1".to_string(),
                trace_id: "trace-1".to_string(),
                parent_span_id: None,
                operation_name: "op1".to_string(),
                start_time: Utc::now(),
                end_time: Utc::now(),
                attributes: HashMap::new(),
                status: SpanStatus::Ok,
            },
            TelemetryData::Metric {
                name: "metric1".to_string(),
                value: 100.0,
                metric_type: MetricType::Gauge,
                timestamp: Utc::now(),
                labels: HashMap::new(),
            },
        ];

        consumer.consume_batch(data).await.unwrap();

        assert_eq!(consumer.count().await, 2);
        let stats = consumer.stats().await.unwrap();
        assert_eq!(stats.spans_consumed, 1);
        assert_eq!(stats.metrics_consumed, 1);
    }

    #[tokio::test]
    async fn test_in_memory_consumer_clear() {
        let consumer = InMemoryConsumer::new();

        let metric = TelemetryData::Metric {
            name: "test".to_string(),
            value: 1.0,
            metric_type: MetricType::Counter,
            timestamp: Utc::now(),
            labels: HashMap::new(),
        };

        consumer.consume(metric).await.unwrap();
        assert_eq!(consumer.count().await, 1);

        consumer.clear().await;
        assert_eq!(consumer.count().await, 0);
    }

    #[tokio::test]
    async fn test_in_memory_consumer_reset_stats() {
        let consumer = InMemoryConsumer::new();

        let span = TelemetryData::Span {
            span_id: "span-1".to_string(),
            trace_id: "trace-1".to_string(),
            parent_span_id: None,
            operation_name: "op1".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            attributes: HashMap::new(),
            status: SpanStatus::Ok,
        };

        consumer.consume(span).await.unwrap();

        let stats = consumer.stats().await.unwrap();
        assert_eq!(stats.spans_consumed, 1);

        consumer.reset_stats().await.unwrap();

        let stats = consumer.stats().await.unwrap();
        assert_eq!(stats.spans_consumed, 0);
        assert_eq!(stats.last_consumption, None);
    }

    #[tokio::test]
    async fn test_noop_consumer() {
        let consumer = NoOpConsumer;

        let span = TelemetryData::Span {
            span_id: "span-1".to_string(),
            trace_id: "trace-1".to_string(),
            parent_span_id: None,
            operation_name: "op1".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            attributes: HashMap::new(),
            status: SpanStatus::Ok,
        };

        consumer.consume(span).await.unwrap();
        let stats = consumer.stats().await.unwrap();
        assert_eq!(stats.spans_consumed, 0);
    }

    #[test]
    fn test_span_status_serialization() {
        let status = SpanStatus::Ok;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"ok\"");

        let deserialized: SpanStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SpanStatus::Ok);
    }

    #[test]
    fn test_metric_type_serialization() {
        let metric_type = MetricType::Counter;
        let json = serde_json::to_string(&metric_type).unwrap();
        assert_eq!(json, "\"counter\"");

        let deserialized: MetricType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, MetricType::Counter);
    }

    #[test]
    fn test_log_level_serialization() {
        let level = LogLevel::Error;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"ERROR\"");

        let deserialized: LogLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LogLevel::Error);
    }
}
