# LLM-memory-graph

[![Crates.io](https://img.shields.io/crates/v/llm-memory-graph.svg)](https://crates.io/crates/llm-memory-graph)
[![Documentation](https://docs.rs/llm-memory-graph/badge.svg)](https://docs.rs/llm-memory-graph)
[![License](https://img.shields.io/crates/l/llm-memory-graph.svg)](https://github.com/globalbusinessadvisors/llm-memory-graph#license)

Graph-based context-tracking and prompt-lineage database for LLM systems.

**llm-memory-graph** provides a persistent, queryable graph database specifically designed for tracking LLM interactions, managing conversation contexts, and tracing prompt lineage through complex multi-agent systems.

## Features

- **Graph-based Storage**: Store conversations, prompts, completions, and relationships as a connected graph
- **Flexible Node Types**: Support for multiple specialized node types:
  - `PromptNode`: Track prompts and their metadata
  - `CompletionNode`: Store LLM responses
  - `ConversationNode`: Organize multi-turn dialogues
  - `ToolInvocationNode`: Track tool/function calls
  - `AgentNode`: Multi-agent system coordination
  - `DocumentNode`, `ContextNode`, `FeedbackNode`, and more
- **Edge Properties**: Rich relationships with metadata, timestamps, and custom properties
- **Query System**: Powerful query interface for traversing and filtering the graph
- **Async Support**: Full async/await support with tokio runtime
- **Streaming Queries**: Efficient streaming for large result sets
- **Persistent Storage**: Built on Sled embedded database
- **Type Safety**: Strongly-typed API with comprehensive error handling
- **Observability**: Built-in metrics and telemetry integration

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
llm-memory-graph = "0.1.0"
```

## Quick Start

### Basic Usage

```rust
use llm_memory_graph::{MemoryGraph, NodeType, EdgeType, CreateNodeRequest};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the graph database
    let graph = MemoryGraph::new("./data/memory_graph")?;

    // Create a prompt node
    let prompt_id = graph.create_node(CreateNodeRequest {
        node_type: NodeType::Prompt,
        content: "What is the capital of France?".to_string(),
        metadata: HashMap::new(),
    })?;

    // Create a completion node
    let completion_id = graph.create_node(CreateNodeRequest {
        node_type: NodeType::Completion,
        content: "The capital of France is Paris.".to_string(),
        metadata: HashMap::new(),
    })?;

    // Link them with an edge
    graph.create_edge(
        prompt_id,
        completion_id,
        EdgeType::Generates,
        HashMap::new(),
    )?;

    // Query the graph
    let nodes = graph.get_neighbors(prompt_id, Some(EdgeType::Generates))?;
    println!("Found {} completion nodes", nodes.len());

    Ok(())
}
```

### Async Streaming Queries

```rust
use llm_memory_graph::{AsyncMemoryGraph, QueryBuilder};
use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let graph = AsyncMemoryGraph::new("./data/memory_graph").await?;

    let query = QueryBuilder::new()
        .node_type(NodeType::Prompt)
        .limit(100)
        .build();

    let mut stream = graph.query_stream(query).await?;

    while let Some(node) = stream.next().await {
        println!("Node: {:?}", node?);
    }

    Ok(())
}
```

### Working with Templates

```rust
use llm_memory_graph::{PromptTemplate, TemplateVariable};

// Create a reusable prompt template
let template = PromptTemplate::new(
    "summarization",
    "Summarize the following text:\n\n{{text}}\n\nSummary:",
    vec![TemplateVariable::new("text", "string", true)],
);

// Render with variables
let mut vars = HashMap::new();
vars.insert("text".to_string(), "Long text to summarize...".to_string());
let rendered = template.render(&vars)?;
```

## Core Concepts

### Node Types

The graph supports multiple specialized node types for different use cases:

- **PromptNode**: User prompts and instructions
- **CompletionNode**: LLM-generated responses
- **ConversationNode**: Multi-turn conversation containers
- **ToolInvocationNode**: Function/tool call records
- **AgentNode**: Multi-agent system coordination
- **DocumentNode**: Source documents and context
- **ContextNode**: Contextual information and metadata
- **FeedbackNode**: Human feedback and ratings

### Edge Types

Relationships between nodes are typed:

- **Generates**: Prompt generates completion
- **References**: Node references another
- **Contains**: Container contains items
- **Triggers**: Action triggers another
- **DependsOn**: Dependency relationship
- **Precedes**: Temporal ordering

### Query System

Powerful query interface with:
- Type filtering
- Time-range queries
- Metadata filtering
- Graph traversal
- Pagination
- Streaming results

## Advanced Features

### Edge Properties

Edges can carry rich metadata:

```rust
let mut edge_metadata = HashMap::new();
edge_metadata.insert("model".to_string(), "gpt-4".to_string());
edge_metadata.insert("temperature".to_string(), "0.7".to_string());
edge_metadata.insert("tokens".to_string(), "150".to_string());

graph.create_edge_with_properties(
    prompt_id,
    completion_id,
    EdgeType::Generates,
    edge_metadata,
)?;
```

### Migration Support

Built-in migration system for schema evolution:

```rust
use llm_memory_graph::migration::{MigrationEngine, Migration};

let mut engine = MigrationEngine::new(graph);
engine.add_migration(Migration::new(
    "001",
    "add_timestamps",
    |graph| {
        // Migration logic
        Ok(())
    },
))?;
engine.run_migrations()?;
```

### Observability Integration

Export metrics to Prometheus:

```rust
use llm_memory_graph::observatory::{Observatory, PrometheusExporter};

let observatory = Observatory::new();
let exporter = PrometheusExporter::new("localhost:9090")?;
observatory.add_exporter(exporter);
```

## Use Cases

- **Conversation Management**: Track multi-turn conversations with full history
- **Prompt Engineering**: Version and test prompt variations
- **Multi-Agent Systems**: Coordinate communication between multiple LLM agents
- **RAG Pipelines**: Track document retrieval and context usage
- **Observability**: Monitor LLM usage patterns and performance
- **Debugging**: Trace prompt lineage and decision paths
- **A/B Testing**: Compare different prompt strategies
- **Compliance**: Audit trails for LLM interactions

## Architecture

Built on proven technologies:
- **Storage**: Sled embedded database for persistence
- **Graph**: Petgraph for in-memory graph operations
- **Serialization**: Multiple formats (JSON, MessagePack, Bincode)
- **Async**: Tokio runtime for concurrent operations
- **Caching**: Moka for intelligent query caching
- **Metrics**: Prometheus integration

## Examples

The repository includes comprehensive examples:

- `simple_chatbot.rs`: Basic chatbot with conversation tracking
- `async_streaming_queries.rs`: Async query patterns
- `edge_properties.rs`: Working with edge metadata
- `prompt_templates.rs`: Template system usage
- `tool_invocations.rs`: Tool call tracking
- `observatory_demo.rs`: Observability integration
- `migration_guide.rs`: Schema migration patterns

Run an example:

```bash
cargo run --example simple_chatbot
```

## Performance

Optimized for production use:
- **Throughput**: 10,000+ events/sec
- **Latency**: p95 < 200ms
- **Caching**: Automatic query result caching
- **Batch Operations**: Bulk insert/update support
- **Streaming**: Memory-efficient result streaming

## Integration

Designed to integrate with the LLM DevOps ecosystem:

- **LLM-Observatory**: Real-time telemetry ingestion
- **LLM-Registry**: Model metadata synchronization
- **LLM-Data-Vault**: Secure storage with encryption
- **gRPC API**: High-performance API server
- **REST API**: HTTP/JSON interface

## Documentation

- [Full API Documentation](https://docs.rs/llm-memory-graph)
- [Examples Directory](https://github.com/globalbusinessadvisors/llm-memory-graph/tree/main/examples)
- [Integration Guide](https://github.com/globalbusinessadvisors/llm-memory-graph/tree/main/docs)

## Contributing

Contributions are welcome! Please ensure:

1. All tests pass: `cargo test`
2. Code is formatted: `cargo fmt`
3. No clippy warnings: `cargo clippy`
4. Add tests for new features

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
