# Observatory Bidirectional Integration

## Overview

This document describes the bidirectional Observatory integration implementation for the LLM Memory Graph project. The integration extends the existing outbound event publishing capabilities with comprehensive inbound telemetry consumption, enabling the memory graph to both publish events and consume telemetry data from external systems.

## Architecture

### Publishing (Outbound) - Existing Functionality

The existing publishing functionality remains unchanged and includes:

- **EventPublisher trait**: Interface for publishing Observatory events
- **InMemoryPublisher**: In-memory implementation for testing
- **NoOpPublisher**: No-operation implementation
- **AsyncEventEmitter**: Async event emission with batching
- **KafkaProducer**: Production-ready Kafka integration

### Consumption (Inbound) - New Functionality

The new consumption capabilities include five major components:

#### 1. TelemetryConsumer (`consumer.rs`)

Core trait and implementations for consuming telemetry data from external systems.

**Key Types:**
- `TelemetryData`: Enum representing different telemetry types (Span, Metric, Log)
- `SpanStatus`: Span execution status (Ok, Error, Unset)
- `MetricType`: Metric types (Counter, Gauge, Histogram, Summary)
- `LogLevel`: Log severity levels
- `ConsumptionStats`: Statistics about consumed telemetry

**Implementations:**
- `InMemoryConsumer`: Stores all consumed data in memory for testing
- `NoOpConsumer`: Discards all data (for disabling consumption)

**Usage Example:**
```rust
use llm_memory_graph::observatory::{InMemoryConsumer, TelemetryConsumer, TelemetryData};

let consumer = InMemoryConsumer::new();
consumer.consume(telemetry_data).await?;
let stats = consumer.stats().await?;
```

#### 2. LineageBuilder (`lineage.rs`)

Constructs lineage chains from distributed trace spans, establishing parent-child relationships and causal dependencies.

**Key Types:**
- `LineageNode`: Single operation in a trace
- `LineageEdge`: Relationship between operations
- `LineageEdgeType`: Types of relationships (ParentChild, Follows, CausedBy, DataFlow)
- `LineageChain`: Complete lineage for a distributed trace

**Features:**
- Automatic parent-child relationship tracking
- Root span identification
- Duration and status aggregation
- Node type inference from operation names

**Usage Example:**
```rust
use llm_memory_graph::observatory::{LineageBuilder, TelemetryData};

let builder = LineageBuilder::new();
builder.process_span(&span_data).await?;

if let Some(chain) = builder.get_chain("trace-id").await {
    println!("Trace has {} spans", chain.nodes.len());
    println!("Total duration: {}ms", chain.total_duration_ms());
}
```

#### 3. TemporalGraphBuilder (`temporal.rs`)

Builds temporal graphs from time-series metrics, enabling correlation analysis and temporal pattern detection.

**Key Types:**
- `TimeSeries`: Time-series data for a specific metric
- `DataPoint`: Individual time-series data point
- `Correlation`: Correlation between two time series
- `TemporalGraph`: Graph showing metric correlations over time
- `TemporalNode`/`TemporalEdge`: Nodes and edges in temporal graphs

**Features:**
- Time-series data management with retention policies
- Pearson correlation coefficient calculation
- Automatic data point cleanup based on retention period
- Downsampling for large datasets
- Statistical aggregations (min, max, average)

**Usage Example:**
```rust
use llm_memory_graph::observatory::TemporalGraphBuilder;
use chrono::{Duration, Utc};

let builder = TemporalGraphBuilder::with_retention(Duration::hours(24));
builder.process_metric(&metric_data).await?;

let now = Utc::now();
let graph = builder.build_graph(
    now - Duration::hours(1),
    now
).await?;

// Find strongly correlated metrics
let correlated = graph.find_correlated_metrics(0.7);
```

#### 4. SpanMapper (`mapping.rs`)

Bidirectional mapper for converting between Observatory telemetry and memory graph entities.

**Key Types:**
- `MappingConfig`: Configuration for span-to-graph conversion
- `MappedEntity`: Graph entity mapped from telemetry
- `MappedEdge`: Edge relationship from lineage
- `MappingResult`: Result of mapping operation

**Features:**
- Operation name pattern matching for node type inference
- Metadata extraction from span attributes
- Trace ID to Session ID mapping
- Bidirectional conversion (telemetry ↔ graph events)
- Caching of span-to-node ID mappings

**Usage Example:**
```rust
use llm_memory_graph::observatory::{SpanMapper, MappingConfig};

let mut mapper = SpanMapper::new();
if let Some(entity) = mapper.map_span(&span_data) {
    println!("Mapped to node type: {:?}", entity.node_type);
    println!("Graph node ID: {}", entity.node_id);
}

// Map lineage chain to graph entities
let result = mapper.map_lineage_chain(&chain);
println!("Mapped {} entities", result.entities.len());
```

#### 5. IngestionPipeline (`ingest.rs`)

Unified pipeline orchestrating all consumption components.

**Key Types:**
- `IngestionConfig`: Pipeline configuration
- `IngestionStats`: Pipeline statistics
- `ProcessingResult`: Result of processing telemetry

**Features:**
- Configurable processing stages (lineage, temporal, mapping)
- Batch and single-item ingestion
- Comprehensive statistics tracking
- Implements `TelemetryConsumer` trait
- Built-in buffering support

**Usage Example:**
```rust
use llm_memory_graph::observatory::{IngestionPipeline, IngestionConfig};

let config = IngestionConfig {
    enable_lineage: true,
    enable_temporal: true,
    enable_mapping: true,
    ..Default::default()
};

let pipeline = IngestionPipeline::with_config(config);
pipeline.ingest(telemetry_data).await?;

// Get lineage chain
let chain = pipeline.get_lineage_chain("trace-id").await;

// Build temporal graph
let graph = pipeline.build_temporal_graph(start, end).await?;

// Get statistics
let stats = pipeline.stats().await;
```

## Configuration

The `ObservatoryConfig` has been extended to support consumption:

```rust
use llm_memory_graph::observatory::ObservatoryConfig;

let config = ObservatoryConfig::new()
    .enabled()                          // Enable publishing
    .with_consumption(true)             // Enable consumption
    .with_lineage(true)                 // Enable lineage building
    .with_temporal(true)                // Enable temporal graphs
    .with_temporal_retention(24)        // 24-hour retention
    .with_batch_size(100)               // Batch size
    .with_flush_interval(1000);         // Flush interval (ms)
```

**New Configuration Options:**
- `enable_consumption`: Enable/disable telemetry consumption
- `enable_lineage`: Enable/disable lineage building from spans
- `enable_temporal`: Enable/disable temporal graph building from metrics
- `temporal_retention_hours`: Retention period for temporal data

## Module Exports

All consumption types are exported from the `observatory` module:

```rust
// Consumption (Inbound)
pub use consumer::{
    ConsumptionStats, InMemoryConsumer, LogLevel, MetricType, NoOpConsumer, SpanStatus,
    TelemetryConsumer, TelemetryData, TraceContext,
};
pub use ingest::{IngestionConfig, IngestionPipeline, IngestionStats, ProcessingResult};
pub use lineage::{LineageBuilder, LineageChain, LineageEdge, LineageEdgeType, LineageNode};
pub use mapping::{MappedEdge, MappedEntity, MappingConfig, MappingResult, SpanMapper};
pub use temporal::{
    Correlation, DataPoint, TemporalGraph, TemporalGraphBuilder, TemporalNode, TimeSeries,
};
```

## Implementation Details

### File Structure

```
crates/llm-memory-graph/src/observatory/
├── config.rs         (Updated - Added consumption config)
├── mod.rs            (Updated - Export new modules)
├── consumer.rs       (NEW - Telemetry consumer trait)
├── lineage.rs        (NEW - Lineage builder)
├── temporal.rs       (NEW - Temporal graph builder)
├── mapping.rs        (NEW - Span mapper)
├── ingest.rs         (NEW - Ingestion pipeline)
├── emitter.rs        (Existing - Event emitter)
├── events.rs         (Existing - Event types)
├── publisher.rs      (Existing - Event publisher)
├── kafka.rs          (Existing - Kafka integration)
├── metrics.rs        (Existing - Metrics collection)
├── prometheus.rs     (Existing - Prometheus integration)
└── streaming.rs      (Existing - Event streaming)
```

### File Sizes

- `consumer.rs`: 14KB (410 lines with tests)
- `lineage.rs`: 17KB (503 lines with tests)
- `temporal.rs`: 18KB (543 lines with tests)
- `mapping.rs`: 19KB (562 lines with tests)
- `ingest.rs`: 17KB (496 lines with tests)
- `config.rs`: 5.5KB (Updated, 193 lines with tests)
- `mod.rs`: 4.5KB (Updated with exports and examples)

**Total new code**: ~85KB, ~2,500 lines including comprehensive tests

### Dependencies

The implementation uses only workspace dependencies:
- `async-trait`: For async trait support
- `tokio`: For async runtime
- `chrono`: For timestamp handling
- `serde`: For serialization
- All core types from `llm-memory-graph-types`

No new external dependencies were introduced.

## Testing

All modules include comprehensive unit tests:

### Consumer Tests (11 tests)
- In-memory consumer span/metric/log handling
- Batch consumption
- Statistics tracking
- Clear and reset operations
- NoOp consumer behavior

### Lineage Tests (11 tests)
- Single span processing
- Parent-child relationships
- Multiple trace handling
- Chain queries and statistics
- Node type inference
- Edge type mapping

### Temporal Tests (10 tests)
- Metric processing
- Time series operations
- Range queries
- Correlation calculation
- Temporal graph building
- Downsampling
- Statistical aggregations

### Mapping Tests (9 tests)
- Span to entity mapping
- Metadata extraction
- Node type inference
- Event to telemetry conversion
- Mapping cache management
- Custom configuration

### Ingestion Tests (10 tests)
- Pipeline ingestion (single and batch)
- Lineage chain retrieval
- Temporal graph building
- Statistics tracking
- Configuration variants
- TelemetryConsumer trait implementation

### Config Tests (5 tests)
- Default configuration
- Builder pattern
- Consumption configuration
- Full configuration builder

**Total tests**: 56 comprehensive unit tests

## Backward Compatibility

The implementation maintains 100% backward compatibility:

1. **Existing functionality unchanged**: All existing publishing features remain exactly as they were
2. **Opt-in consumption**: Consumption is disabled by default (`enable_consumption: false`)
3. **No breaking changes**: All new types are additions, no modifications to existing APIs
4. **Separate modules**: New functionality in separate modules (consumer, lineage, temporal, mapping, ingest)

## Use Cases

### 1. Distributed Trace Analysis

```rust
let pipeline = IngestionPipeline::new();

// Ingest spans from distributed trace
for span in trace_spans {
    pipeline.ingest(span).await?;
}

// Get complete lineage chain
if let Some(chain) = pipeline.get_lineage_chain(&trace_id).await {
    // Analyze trace structure
    println!("Root operations: {:?}", chain.roots);
    println!("Total duration: {}ms", chain.total_duration_ms());

    // Map to graph entities
    let mapping = pipeline.map_lineage_chain(&chain).await;
    for entity in mapping.entities {
        // Create graph nodes from lineage
    }
}
```

### 2. Metric Correlation Analysis

```rust
let pipeline = IngestionPipeline::new();

// Ingest metrics over time
for metric in metrics {
    pipeline.ingest(metric).await?;
}

// Build temporal correlation graph
let graph = pipeline.build_temporal_graph(
    start_time,
    end_time
).await?;

// Find correlated metrics
let correlated = graph.find_correlated_metrics(0.8);
for edge in correlated {
    println!(
        "{} ↔ {} (correlation: {:.2})",
        edge.from_metric,
        edge.to_metric,
        edge.correlation
    );
}
```

### 3. Bidirectional Event Flow

```rust
// Publishing (outbound)
let publisher = InMemoryPublisher::new();
graph.emit_event(MemoryGraphEvent::NodeCreated { ... }).await?;

// Consumption (inbound)
let mapper = SpanMapper::new();
let telemetry = mapper.event_to_telemetry(&event);

// Create lineage from external traces
let pipeline = IngestionPipeline::new();
pipeline.ingest(external_span).await?;
```

## Performance Considerations

1. **Memory Management**:
   - In-memory implementations use Arc<RwLock<>> for thread-safe access
   - Temporal builder includes automatic cleanup based on retention period

2. **Caching**:
   - SpanMapper caches span-to-node ID mappings to avoid regeneration
   - Can be cleared explicitly if needed

3. **Batch Processing**:
   - All consumers support batch operations for efficiency
   - Pipeline supports buffering for controlled processing

4. **Lock Contention**:
   - Read-write locks used judiciously
   - Statistics updates are atomic where possible

## Future Enhancements

Potential future additions (not included in this implementation):

1. **Log Processing**: Extract graph relationships from structured logs
2. **Lag Analysis**: Calculate time lags in temporal correlations
3. **Anomaly Detection**: Detect anomalies in lineage chains and metrics
4. **Graph Persistence**: Persist lineage chains and temporal graphs to storage
5. **Advanced Correlation**: Support for cross-correlation and other statistical methods
6. **Real-time Streaming**: Direct integration with streaming telemetry sources

## Summary

This implementation successfully extends the Observatory module with comprehensive bidirectional capabilities:

- ✅ **5 new modules** with full functionality
- ✅ **56 unit tests** covering all features
- ✅ **100% backward compatible** with existing code
- ✅ **Zero new dependencies** beyond workspace
- ✅ **Comprehensive documentation** in all modules
- ✅ **Production-ready** design patterns (Arc, RwLock, async/await)
- ✅ **Flexible configuration** with builder pattern
- ✅ **Type-safe** throughout with strong typing

The implementation provides a solid foundation for consuming telemetry data, building lineage chains, analyzing temporal patterns, and mapping between Observatory events and memory graph entities.
