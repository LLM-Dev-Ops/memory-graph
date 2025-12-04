//! Temporal graph builder for correlating metrics over time
//!
//! This module provides functionality to build temporal graphs from time-series
//! metrics, enabling correlation analysis and temporal pattern detection.

use super::consumer::{MetricType, TelemetryData};
use crate::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;

/// A time-series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Value
    pub value: f64,
    /// Labels/dimensions
    pub labels: HashMap<String, String>,
}

/// A time series for a specific metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Data points ordered by timestamp
    pub points: BTreeMap<DateTime<Utc>, DataPoint>,
}

impl TimeSeries {
    /// Create a new time series
    pub fn new(name: String, metric_type: MetricType) -> Self {
        Self {
            name,
            metric_type,
            points: BTreeMap::new(),
        }
    }

    /// Add a data point
    pub fn add_point(&mut self, point: DataPoint) {
        self.points.insert(point.timestamp, point);
    }

    /// Get points within a time range
    pub fn range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&DataPoint> {
        self.points
            .range(start..=end)
            .map(|(_, point)| point)
            .collect()
    }

    /// Get the latest data point
    pub fn latest(&self) -> Option<&DataPoint> {
        self.points.values().next_back()
    }

    /// Calculate average value over the time series
    pub fn average(&self) -> Option<f64> {
        if self.points.is_empty() {
            return None;
        }

        let sum: f64 = self.points.values().map(|p| p.value).sum();
        Some(sum / self.points.len() as f64)
    }

    /// Calculate minimum value
    pub fn min(&self) -> Option<f64> {
        self.points.values().map(|p| p.value).min_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Calculate maximum value
    pub fn max(&self) -> Option<f64> {
        self.points.values().map(|p| p.value).max_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Get the number of data points
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Check if the time series is empty
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Downsample the time series to a target number of points
    pub fn downsample(&self, target_points: usize) -> TimeSeries {
        if self.points.len() <= target_points {
            return self.clone();
        }

        let mut downsampled = TimeSeries::new(self.name.clone(), self.metric_type);
        let step = self.points.len() / target_points;

        for (i, (_, point)) in self.points.iter().enumerate() {
            if i % step == 0 {
                downsampled.add_point(point.clone());
            }
        }

        downsampled
    }
}

/// Correlation between two time series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correlation {
    /// First metric name
    pub metric_a: String,
    /// Second metric name
    pub metric_b: String,
    /// Correlation coefficient (-1 to 1)
    pub coefficient: f64,
    /// Time range for correlation
    pub start_time: DateTime<Utc>,
    /// End time for correlation
    pub end_time: DateTime<Utc>,
    /// Number of data points used
    pub sample_size: usize,
}

/// Temporal graph node representing a metric at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalNode {
    /// Metric name
    pub metric: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Value at this time
    pub value: f64,
    /// Labels
    pub labels: HashMap<String, String>,
}

/// Temporal graph edge representing correlation or causation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalEdge {
    /// Source metric
    pub from_metric: String,
    /// Target metric
    pub to_metric: String,
    /// Correlation coefficient
    pub correlation: f64,
    /// Time lag in milliseconds (if any)
    pub lag_ms: i64,
}

/// A temporal graph showing metric correlations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalGraph {
    /// Nodes in the temporal graph
    pub nodes: Vec<TemporalNode>,
    /// Edges showing correlations
    pub edges: Vec<TemporalEdge>,
    /// Time range of the graph
    pub start_time: DateTime<Utc>,
    /// End time of the graph
    pub end_time: DateTime<Utc>,
}

impl TemporalGraph {
    /// Create a new temporal graph
    pub fn new(start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            start_time,
            end_time,
        }
    }

    /// Add a node to the temporal graph
    pub fn add_node(&mut self, node: TemporalNode) {
        self.nodes.push(node);
    }

    /// Add an edge to the temporal graph
    pub fn add_edge(&mut self, edge: TemporalEdge) {
        self.edges.push(edge);
    }

    /// Find strongly correlated metrics (|correlation| > threshold)
    pub fn find_correlated_metrics(&self, threshold: f64) -> Vec<&TemporalEdge> {
        self.edges
            .iter()
            .filter(|e| e.correlation.abs() > threshold)
            .collect()
    }
}

/// Builder for constructing temporal graphs from metrics
pub struct TemporalGraphBuilder {
    /// Time series data indexed by metric name
    series: Arc<RwLock<HashMap<String, TimeSeries>>>,
    /// Correlation cache
    correlations: Arc<RwLock<Vec<Correlation>>>,
    /// Retention period for metrics
    retention: Duration,
}

impl TemporalGraphBuilder {
    /// Create a new temporal graph builder
    pub fn new() -> Self {
        Self::with_retention(Duration::hours(24))
    }

    /// Create a new temporal graph builder with custom retention
    pub fn with_retention(retention: Duration) -> Self {
        Self {
            series: Arc::new(RwLock::new(HashMap::new())),
            correlations: Arc::new(RwLock::new(Vec::new())),
            retention,
        }
    }

    /// Process a metric and add it to the appropriate time series
    pub async fn process_metric(&self, data: &TelemetryData) -> Result<()> {
        if let TelemetryData::Metric {
            name,
            value,
            metric_type,
            timestamp,
            labels,
        } = data
        {
            let point = DataPoint {
                timestamp: *timestamp,
                value: *value,
                labels: labels.clone(),
            };

            let mut series = self.series.write().await;
            let ts = series
                .entry(name.clone())
                .or_insert_with(|| TimeSeries::new(name.clone(), *metric_type));

            ts.add_point(point);

            // Clean up old data points outside retention period
            let cutoff = Utc::now() - self.retention;
            ts.points.retain(|timestamp, _| *timestamp >= cutoff);
        }

        Ok(())
    }

    /// Get a time series by name
    pub async fn get_series(&self, name: &str) -> Option<TimeSeries> {
        self.series.read().await.get(name).cloned()
    }

    /// Get all time series
    pub async fn get_all_series(&self) -> Vec<TimeSeries> {
        self.series.read().await.values().cloned().collect()
    }

    /// Calculate correlation between two metrics
    pub async fn calculate_correlation(
        &self,
        metric_a: &str,
        metric_b: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Option<Correlation>> {
        let series = self.series.read().await;

        let ts_a = match series.get(metric_a) {
            Some(ts) => ts,
            None => return Ok(None),
        };

        let ts_b = match series.get(metric_b) {
            Some(ts) => ts,
            None => return Ok(None),
        };

        let points_a = ts_a.range(start, end);
        let points_b = ts_b.range(start, end);

        if points_a.len() < 2 || points_b.len() < 2 {
            return Ok(None);
        }

        // Simple Pearson correlation coefficient calculation
        let values_a: Vec<f64> = points_a.iter().map(|p| p.value).collect();
        let values_b: Vec<f64> = points_b.iter().map(|p| p.value).collect();

        let n = values_a.len().min(values_b.len()) as f64;
        let mean_a: f64 = values_a.iter().take(n as usize).sum::<f64>() / n;
        let mean_b: f64 = values_b.iter().take(n as usize).sum::<f64>() / n;

        let mut covariance = 0.0;
        let mut variance_a = 0.0;
        let mut variance_b = 0.0;

        for i in 0..(n as usize) {
            let diff_a = values_a[i] - mean_a;
            let diff_b = values_b[i] - mean_b;
            covariance += diff_a * diff_b;
            variance_a += diff_a * diff_a;
            variance_b += diff_b * diff_b;
        }

        let coefficient = if variance_a > 0.0 && variance_b > 0.0 {
            covariance / (variance_a.sqrt() * variance_b.sqrt())
        } else {
            0.0
        };

        let correlation = Correlation {
            metric_a: metric_a.to_string(),
            metric_b: metric_b.to_string(),
            coefficient,
            start_time: start,
            end_time: end,
            sample_size: n as usize,
        };

        // Cache the correlation
        self.correlations.write().await.push(correlation.clone());

        Ok(Some(correlation))
    }

    /// Build a temporal graph for a time range
    pub async fn build_graph(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<TemporalGraph> {
        let mut graph = TemporalGraph::new(start, end);

        let series = self.series.read().await;

        // Add nodes for each metric's data points in the range
        for ts in series.values() {
            for point in ts.range(start, end) {
                graph.add_node(TemporalNode {
                    metric: ts.name.clone(),
                    timestamp: point.timestamp,
                    value: point.value,
                    labels: point.labels.clone(),
                });
            }
        }

        drop(series);

        // Calculate correlations and add edges
        let metrics: Vec<String> = self.series.read().await.keys().cloned().collect();

        for i in 0..metrics.len() {
            for j in (i + 1)..metrics.len() {
                if let Some(corr) = self
                    .calculate_correlation(&metrics[i], &metrics[j], start, end)
                    .await?
                {
                    if corr.coefficient.abs() > 0.5 {
                        // Only add significant correlations
                        graph.add_edge(TemporalEdge {
                            from_metric: metrics[i].clone(),
                            to_metric: metrics[j].clone(),
                            correlation: corr.coefficient,
                            lag_ms: 0, // Could be enhanced with lag analysis
                        });
                    }
                }
            }
        }

        Ok(graph)
    }

    /// Get cached correlations
    pub async fn get_correlations(&self) -> Vec<Correlation> {
        self.correlations.read().await.clone()
    }

    /// Clear all cached correlations
    pub async fn clear_correlations(&self) {
        self.correlations.write().await.clear();
    }

    /// Clear all time series data
    pub async fn clear(&self) {
        self.series.write().await.clear();
        self.correlations.write().await.clear();
    }

    /// Get the number of tracked metrics
    pub async fn metric_count(&self) -> usize {
        self.series.read().await.len()
    }
}

impl Default for TemporalGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metric(name: &str, value: f64, timestamp: DateTime<Utc>) -> TelemetryData {
        TelemetryData::Metric {
            name: name.to_string(),
            value,
            metric_type: MetricType::Gauge,
            timestamp,
            labels: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_temporal_graph_builder_process_metric() {
        let builder = TemporalGraphBuilder::new();

        let metric = create_test_metric("cpu.usage", 0.75, Utc::now());
        builder.process_metric(&metric).await.unwrap();

        assert_eq!(builder.metric_count().await, 1);

        let series = builder.get_series("cpu.usage").await.unwrap();
        assert_eq!(series.len(), 1);
    }

    #[tokio::test]
    async fn test_time_series_operations() {
        let mut series = TimeSeries::new("test".to_string(), MetricType::Gauge);

        let now = Utc::now();
        series.add_point(DataPoint {
            timestamp: now,
            value: 10.0,
            labels: HashMap::new(),
        });

        series.add_point(DataPoint {
            timestamp: now + Duration::seconds(1),
            value: 20.0,
            labels: HashMap::new(),
        });

        series.add_point(DataPoint {
            timestamp: now + Duration::seconds(2),
            value: 30.0,
            labels: HashMap::new(),
        });

        assert_eq!(series.len(), 3);
        assert_eq!(series.average(), Some(20.0));
        assert_eq!(series.min(), Some(10.0));
        assert_eq!(series.max(), Some(30.0));

        let latest = series.latest().unwrap();
        assert_eq!(latest.value, 30.0);
    }

    #[tokio::test]
    async fn test_time_series_range() {
        let mut series = TimeSeries::new("test".to_string(), MetricType::Counter);

        let start = Utc::now();
        for i in 0..10 {
            series.add_point(DataPoint {
                timestamp: start + Duration::seconds(i),
                value: i as f64,
                labels: HashMap::new(),
            });
        }

        let range_start = start + Duration::seconds(3);
        let range_end = start + Duration::seconds(7);
        let points = series.range(range_start, range_end);

        assert_eq!(points.len(), 5); // 3, 4, 5, 6, 7
    }

    #[tokio::test]
    async fn test_correlation_calculation() {
        let builder = TemporalGraphBuilder::new();

        let now = Utc::now();

        // Create two perfectly correlated metrics
        for i in 0..10 {
            let timestamp = now + Duration::seconds(i);
            let metric_a = create_test_metric("metric_a", i as f64, timestamp);
            let metric_b = create_test_metric("metric_b", i as f64 * 2.0, timestamp);

            builder.process_metric(&metric_a).await.unwrap();
            builder.process_metric(&metric_b).await.unwrap();
        }

        let corr = builder
            .calculate_correlation("metric_a", "metric_b", now, now + Duration::seconds(10))
            .await
            .unwrap()
            .unwrap();

        // Should be perfectly correlated
        assert!((corr.coefficient - 1.0).abs() < 0.01);
        assert_eq!(corr.sample_size, 10);
    }

    #[tokio::test]
    async fn test_build_temporal_graph() {
        let builder = TemporalGraphBuilder::new();

        let now = Utc::now();

        for i in 0..5 {
            let timestamp = now + Duration::seconds(i);
            let metric_a = create_test_metric("metric_a", i as f64, timestamp);
            let metric_b = create_test_metric("metric_b", i as f64 * 1.5, timestamp);

            builder.process_metric(&metric_a).await.unwrap();
            builder.process_metric(&metric_b).await.unwrap();
        }

        let graph = builder
            .build_graph(now, now + Duration::seconds(5))
            .await
            .unwrap();

        assert_eq!(graph.nodes.len(), 10); // 5 points for each of 2 metrics
        assert!(graph.edges.len() > 0); // Should have correlation edge
    }

    #[tokio::test]
    async fn test_temporal_graph_builder_clear() {
        let builder = TemporalGraphBuilder::new();

        let metric = create_test_metric("test", 1.0, Utc::now());
        builder.process_metric(&metric).await.unwrap();

        assert_eq!(builder.metric_count().await, 1);

        builder.clear().await;
        assert_eq!(builder.metric_count().await, 0);
    }

    #[test]
    fn test_time_series_downsample() {
        let mut series = TimeSeries::new("test".to_string(), MetricType::Gauge);

        let now = Utc::now();
        for i in 0..100 {
            series.add_point(DataPoint {
                timestamp: now + Duration::seconds(i),
                value: i as f64,
                labels: HashMap::new(),
            });
        }

        let downsampled = series.downsample(10);
        assert!(downsampled.len() <= 10);
        assert!(downsampled.len() > 0);
    }

    #[test]
    fn test_temporal_graph_find_correlated() {
        let mut graph = TemporalGraph::new(Utc::now(), Utc::now() + Duration::hours(1));

        graph.add_edge(TemporalEdge {
            from_metric: "a".to_string(),
            to_metric: "b".to_string(),
            correlation: 0.9,
            lag_ms: 0,
        });

        graph.add_edge(TemporalEdge {
            from_metric: "c".to_string(),
            to_metric: "d".to_string(),
            correlation: 0.3,
            lag_ms: 0,
        });

        let correlated = graph.find_correlated_metrics(0.7);
        assert_eq!(correlated.len(), 1);
        assert_eq!(correlated[0].correlation, 0.9);
    }
}
