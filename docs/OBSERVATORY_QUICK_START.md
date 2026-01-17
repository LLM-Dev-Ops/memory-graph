# Observatory Bidirectional Integration - Quick Start Guide

## Installation

The Observatory integration is built into the `llm-memory-graph` crate. No additional dependencies needed.

```rust
use llm_memory_graph::observatory::*;
```

## Basic Usage

### 1. Publishing Events (Outbound)

```rust
use llm_memory_graph::observatory::{InMemoryPublisher, ObservatoryConfig};
use std::sync::Arc;

// Create publisher
let publisher = Arc::new(InMemoryPublisher::new());

// Configure Observatory
let config = ObservatoryConfig::new()
    .enabled()
    .with_batch_size(50);

// Use with memory graph (existing functionality)
let graph = AsyncMemoryGraph::with_observatory(
    Config::default(),
    Some(publisher.clone()),
    config
).await?;

// Events are automatically published
let session = graph.create_session().await?;

// Retrieve published events
let events = publisher.get_events().await;
```

### 2. Consuming Telemetry (Inbound)

#### Simple Consumer

```rust
use llm_memory_graph::observatory::{InMemoryConsumer, TelemetryConsumer, TelemetryData};
use chrono::Utc;
use std::collections::HashMap;

// Create consumer
let consumer = InMemoryConsumer::new();

// Consume a span
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

consumer.consume(span).await?;

// Get statistics
let stats = consumer.stats().await?;
println!("Consumed {} spans", stats.spans_consumed);
```

#### Full Ingestion Pipeline

```rust
use llm_memory_graph::observatory::{IngestionPipeline, IngestionConfig};

// Create pipeline with default config
let pipeline = IngestionPipeline::new();

// Or with custom config
let config = IngestionConfig {
    enable_lineage: true,
    enable_temporal: true,
    enable_mapping: true,
    temporal_retention_hours: 48,
    ..Default::default()
};
let pipeline = IngestionPipeline::with_config(config);

// Ingest telemetry
pipeline.ingest(span_data).await?;
pipeline.ingest(metric_data).await?;

// Get statistics
let stats = pipeline.stats().await;
println!("Processed {} items", stats.total_ingested);
```

### 3. Building Lineage Chains

```rust
use llm_memory_graph::observatory::{LineageBuilder, TelemetryData};

let builder = LineageBuilder::new();

// Process spans (automatically builds lineage)
for span in trace_spans {
    builder.process_span(&span).await?;
}

// Get lineage chain
if let Some(chain) = builder.get_chain("trace-id").await {
    println!("Trace has {} spans", chain.nodes.len());
    println!("Root spans: {:?}", chain.roots);
    println!("Total duration: {}ms", chain.total_duration_ms());

    // Analyze status
    let status_counts = chain.count_by_status();
    println!("Errors: {:?}", status_counts.get(&SpanStatus::Error));
}
```

### 4. Building Temporal Graphs

```rust
use llm_memory_graph::observatory::TemporalGraphBuilder;
use chrono::{Duration, Utc};

let builder = TemporalGraphBuilder::with_retention(Duration::hours(24));

// Process metrics
for metric in metrics {
    builder.process_metric(&metric).await?;
}

// Build temporal graph
let now = Utc::now();
let graph = builder.build_graph(
    now - Duration::hours(1),
    now
).await?;

// Find correlated metrics
let correlated = graph.find_correlated_metrics(0.7);
for edge in correlated {
    println!(
        "{} ↔ {} (r={:.2})",
        edge.from_metric,
        edge.to_metric,
        edge.correlation
    );
}

// Get time series
if let Some(series) = builder.get_series("cpu.usage").await {
    println!("Average: {:.2}", series.average().unwrap_or(0.0));
    println!("Min: {:.2}", series.min().unwrap_or(0.0));
    println!("Max: {:.2}", series.max().unwrap_or(0.0));
}
```

### 5. Mapping Spans to Graph Entities

```rust
use llm_memory_graph::observatory::{SpanMapper, MappingConfig};

let mut mapper = SpanMapper::new();

// Map a single span
if let Some(entity) = mapper.map_span(&span_data) {
    println!("Node ID: {}", entity.node_id);
    println!("Node Type: {:?}", entity.node_type);
    println!("Session ID: {:?}", entity.session_id);
    println!("Metadata: {:?}", entity.metadata);
}

// Map entire lineage chain
let result = mapper.map_lineage_chain(&chain);
println!("Mapped {} entities", result.entities.len());
println!("Created {} edges", result.edges.len());

if !result.is_success() {
    println!("Errors: {:?}", result.errors);
}
```

### 6. Bidirectional Conversion

```rust
use llm_memory_graph::observatory::{SpanMapper, MemoryGraphEvent};

let mapper = SpanMapper::new();

// Convert graph event to telemetry
let event = MemoryGraphEvent::NodeCreated {
    node_id: NodeId::new(),
    node_type: NodeType::Prompt,
    session_id: Some(SessionId::new()),
    timestamp: Utc::now(),
    metadata: HashMap::new(),
};

if let Some(telemetry) = mapper.event_to_telemetry(&event) {
    // Use telemetry data (e.g., send to external system)
}
```

## Configuration Examples

### Publishing Only
```rust
let config = ObservatoryConfig::new()
    .enabled()
    .with_batch_size(100)
    .with_flush_interval(1000);
```

### Consumption Only
```rust
let config = ObservatoryConfig::new()
    .with_consumption(true)
    .with_lineage(true)
    .with_temporal(true)
    .with_temporal_retention(24);
```

### Bidirectional (Both)
```rust
let config = ObservatoryConfig::new()
    .enabled()                       // Enable publishing
    .with_consumption(true)          // Enable consumption
    .with_batch_size(100)
    .with_flush_interval(1000)
    .with_lineage(true)
    .with_temporal(true)
    .with_temporal_retention(48);
```

### Custom Mapping Configuration
```rust
use std::collections::HashMap;

let mut patterns = HashMap::new();
patterns.insert("custom.operation".to_string(), NodeType::Tool);

let mapping_config = MappingConfig {
    operation_patterns: patterns,
    metadata_keys: vec!["custom_key".to_string()],
    create_lineage_edges: true,
    trace_to_session: true,
};

let mapper = SpanMapper::with_config(mapping_config);
```

## Common Patterns

### Pattern 1: Trace Analysis Pipeline

```rust
async fn analyze_trace(trace_id: &str, spans: Vec<TelemetryData>) -> Result<()> {
    let pipeline = IngestionPipeline::new();

    // Ingest all spans
    for span in spans {
        pipeline.ingest(span).await?;
    }

    // Get lineage chain
    let chain = pipeline.get_lineage_chain(trace_id)
        .await
        .ok_or_else(|| Error::msg("Chain not found"))?;

    // Map to graph entities
    let mapping = pipeline.map_lineage_chain(&chain).await;

    // Create graph nodes and edges
    for entity in mapping.entities {
        // graph.create_node(entity).await?;
    }

    for edge in mapping.edges {
        // graph.create_edge(edge).await?;
    }

    Ok(())
}
```

### Pattern 2: Real-time Metric Correlation

```rust
async fn monitor_metrics(metrics: Vec<TelemetryData>) -> Result<()> {
    let builder = TemporalGraphBuilder::new();

    // Process metrics
    for metric in metrics {
        builder.process_metric(&metric).await?;
    }

    // Build correlation graph
    let now = Utc::now();
    let graph = builder.build_graph(
        now - Duration::minutes(5),
        now
    ).await?;

    // Alert on strong correlations
    for edge in graph.find_correlated_metrics(0.9) {
        println!(
            "ALERT: Strong correlation between {} and {}",
            edge.from_metric,
            edge.to_metric
        );
    }

    Ok(())
}
```

### Pattern 3: Batch Processing

```rust
async fn process_batch(batch: Vec<TelemetryData>) -> Result<()> {
    let pipeline = IngestionPipeline::new();

    // Process entire batch
    let results = pipeline.ingest_batch(batch).await?;

    // Check for errors
    let errors: Vec<_> = results.iter()
        .filter(|r| !r.success)
        .collect();

    if !errors.is_empty() {
        println!("Batch had {} errors", errors.len());
        for result in errors {
            println!("Errors: {:?}", result.errors);
        }
    }

    // Get statistics
    let stats = pipeline.stats().await;
    println!("Pipeline stats: {:#?}", stats);

    Ok(())
}
```

## Testing Examples

### Unit Testing with InMemoryConsumer

```rust
#[tokio::test]
async fn test_my_telemetry_processor() {
    let consumer = InMemoryConsumer::new();

    // Process telemetry
    let span = create_test_span();
    consumer.consume(span).await.unwrap();

    // Verify
    assert_eq!(consumer.count().await, 1);
    let spans = consumer.get_spans().await;
    assert_eq!(spans.len(), 1);

    let stats = consumer.stats().await.unwrap();
    assert_eq!(stats.spans_consumed, 1);
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_end_to_end_pipeline() {
    let pipeline = IngestionPipeline::new();

    // Ingest trace spans
    let spans = vec![
        create_span("span-1", "trace-1", None),
        create_span("span-2", "trace-1", Some("span-1")),
    ];

    for span in spans {
        pipeline.ingest(span).await.unwrap();
    }

    // Verify lineage chain
    let chain = pipeline.get_lineage_chain("trace-1")
        .await
        .expect("Chain should exist");

    assert_eq!(chain.nodes.len(), 2);
    assert_eq!(chain.edges.len(), 1);

    // Verify mapping
    let mapping = pipeline.map_lineage_chain(&chain).await;
    assert_eq!(mapping.entities.len(), 2);
    assert!(mapping.is_success());
}
```

## Troubleshooting

### Issue: Lineage chain not found

```rust
// Make sure to process spans before querying
builder.process_span(&span).await?;
let chain = builder.get_chain("trace-id").await;
```

### Issue: Temporal data missing

```rust
// Check retention period
let builder = TemporalGraphBuilder::with_retention(Duration::hours(24));

// Verify data is within retention window
let now = Utc::now();
let cutoff = now - Duration::hours(24);
// Only data after cutoff will be available
```

### Issue: Mapping errors

```rust
let result = mapper.map_lineage_chain(&chain);
if !result.is_success() {
    for error in result.errors {
        println!("Mapping error: {}", error);
    }
}
```

### Issue: Statistics not updating

```rust
// Ensure you're using the same consumer instance
let consumer = Arc::new(InMemoryConsumer::new());
consumer.consume(data).await?;
let stats = consumer.stats().await?; // Use same instance
```

## Best Practices

1. **Use Arc for shared consumers**: Wrap consumers in Arc when sharing across threads
2. **Configure retention appropriately**: Set temporal retention based on your needs
3. **Handle errors gracefully**: Check `MappingResult.is_success()` and process errors
4. **Clear caches periodically**: Call `clear_cache()` on SpanMapper if needed
5. **Use batch operations**: Prefer `ingest_batch()` over multiple `ingest()` calls
6. **Monitor statistics**: Regularly check `stats()` for insights and debugging

## Additional Resources

- Full documentation: `/workspaces/memory-graph/OBSERVATORY_BIDIRECTIONAL_INTEGRATION.md`
- Module documentation: See inline docs in each source file
- Examples: See test cases in each module
