# LLM Memory Graph Documentation

Complete documentation for the LLM Memory Graph system.

## Quick Navigation

### Getting Started
- [Quick Start Guide](guides/quickstart.md) - Get up and running in minutes
- [Installation Guide](#installation)
- [Examples](EXAMPLES.md) - Practical code examples

### API Documentation
- [API Reference](API.md) - Complete API documentation for all SDKs
- [TypeScript API](typescript/index.html) - TypeDoc-generated TypeScript documentation
- [Rust API](rust/llm_memory_graph/index.html) - Rustdoc-generated Rust documentation
- [gRPC API](API.md#grpc-api) - Protocol Buffers interface

### CLI Documentation
- [CLI Reference](cli/README.md) - Command-line interface overview
- [CLI Commands](cli/commands.md) - Detailed command reference
- [CLI Examples](cli/examples.md) - Common CLI usage patterns

### Guides
- [Quick Start](guides/quickstart.md) - First steps with LLM Memory Graph
- [Advanced Guide](guides/advanced.md) - Advanced features and patterns
- [Authentication](guides/authentication.md) - Security and authentication setup

### Architecture & Development
- [Architecture Overview](ARCHITECTURE.md) - System architecture
- [Component Diagrams](COMPONENT_DIAGRAMS.md) - Visual architecture
- [Deployment Guide](DEPLOYMENT_GUIDE.md) - Deployment instructions
- [Integration Guide](INTEGRATION_IMPLEMENTATION_GUIDE.md) - Integration patterns

### Contributing
- [Contributing Guidelines](../CONTRIBUTING.md) - How to contribute
- [Changelog](../CHANGELOG.md) - Version history
- [Code Examples](EXAMPLES.md) - Usage examples

## Documentation Structure

```
docs/
├── README.md                    # This file - documentation index
├── API.md                       # Complete API reference
├── EXAMPLES.md                  # Usage examples
├── typescript/                  # TypeDoc output (generated)
│   └── index.html              # TypeScript API docs
├── rust/                        # Rustdoc output (generated)
│   └── llm_memory_graph/       # Rust API docs
├── cli/                         # CLI documentation
│   ├── README.md               # CLI overview
│   ├── commands.md             # Command reference
│   └── examples.md             # CLI examples
└── guides/                      # User guides
    ├── quickstart.md           # Getting started
    ├── advanced.md             # Advanced topics
    └── authentication.md       # Security setup
```

## Installation

### Server Installation

#### From Source
```bash
git clone https://github.com/globalbusinessadvisors/llm-memory-graph.git
cd llm-memory-graph
cargo build --release
```

#### Using Cargo
```bash
cargo install --path crates/llm-memory-graph-cli
```

#### Using Docker
```bash
docker pull ghcr.io/globalbusinessadvisors/llm-memory-graph:latest
```

### Client Installation

#### TypeScript/JavaScript
```bash
npm install @llm-dev-ops/llm-memory-graph-client
```

#### Rust
Add to `Cargo.toml`:
```toml
[dependencies]
llm-memory-graph-client = "0.1.0"
```

## Core Concepts

### Sessions
Sessions organize related interactions. Each session can contain multiple prompts, responses, and tool invocations.

### Nodes
Nodes represent entities in the graph:
- **Prompt Nodes** - User inputs or system prompts
- **Response Nodes** - LLM-generated responses
- **Tool Invocation Nodes** - Tool/function calls
- **Template Nodes** - Reusable prompt templates
- **Agent Nodes** - AI agents in the system

### Edges
Edges represent relationships between nodes:
- `BELONGS_TO` - Node belongs to session
- `RESPONDS_TO` - Response to prompt
- `FOLLOWS` - Sequential relationship
- `INVOKES` - Tool invocation
- `HANDLED_BY` - Agent handling

## Quick Example

### TypeScript
```typescript
import { MemoryGraphClient } from '@llm-dev-ops/llm-memory-graph-client';

const client = new MemoryGraphClient({ address: 'localhost:50051' });

// Create session
const session = await client.createSession();

// Add prompt
const prompt = await client.addPrompt({
  sessionId: session.id,
  content: 'What is TypeScript?'
});

// Add response
const response = await client.addResponse({
  promptId: prompt.id,
  content: 'TypeScript is a typed superset of JavaScript.'
});

await client.close();
```

### Rust
```rust
use llm_memory_graph_client::{Client, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig {
        endpoint: "http://localhost:50051".to_string(),
        ..Default::default()
    };

    let mut client = Client::new(config).await?;

    // Create session
    let session = client.create_session(None).await?;

    // Add prompt
    let prompt = client.add_prompt(
        &session.id,
        "What is Rust?",
        None
    ).await?;

    // Add response
    let response = client.add_response(
        &prompt.id,
        "Rust is a systems programming language.",
        None,
        None
    ).await?;

    Ok(())
}
```

### CLI
```bash
# Start server
llm-memory-graph server start

# View statistics
llm-memory-graph stats

# Query nodes
llm-memory-graph query -t prompt -l 10

# Export data
llm-memory-graph export database -o backup.json
```

## Features

### Core Features
- Graph-based conversation tracking
- Session management
- Prompt lineage tracking
- Tool invocation monitoring
- Template system
- Multi-agent workflows

### Client Features
- TypeScript/JavaScript client
- Rust client
- Automatic retry logic
- Connection pooling
- Error handling
- Health monitoring

### Advanced Features
- Event streaming
- Real-time metrics
- Export/import (JSON, MessagePack)
- Plugin system
- Vault integration
- Observatory monitoring

## Common Use Cases

### Chatbot Integration
Track conversations, prompts, and responses in chatbot applications.

[See Example](EXAMPLES.md#chatbot-integration)

### Agent Workflows
Coordinate multi-agent systems with complex tool usage.

[See Example](EXAMPLES.md#agent-workflows)

### Prompt Analytics
Analyze prompt performance, token usage, and conversation patterns.

[See Example](EXAMPLES.md#analytics-and-reporting)

### Template Management
Create and manage reusable prompt templates with variables.

[See Example](EXAMPLES.md#template-usage)

## API Overview

### TypeScript Client
```typescript
class MemoryGraphClient {
  // Session Management
  createSession(options?: CreateSessionOptions): Promise<Session>
  getSession(sessionId: string): Promise<Session>
  closeSession(sessionId: string): Promise<void>

  // Node Operations
  addPrompt(request: AddPromptRequest): Promise<PromptNode>
  addResponse(request: AddResponseRequest): Promise<ResponseNode>
  addToolInvocation(request: AddToolInvocationRequest): Promise<ToolInvocationNode>

  // Querying
  queryNodes(options: QueryOptions): Promise<QueryResult>
  getSessionNodes(sessionId: string, limit?: number, offset?: number): Promise<Node[]>

  // Templates
  createTemplate(request: CreateTemplateRequest): Promise<TemplateNode>
  instantiateTemplate(request: InstantiateTemplateRequest): Promise<PromptNode>

  // Streaming
  streamEvents(options: EventStreamOptions): Promise<void>

  // Health & Metrics
  checkHealth(): Promise<HealthResponse>
  getMetrics(): Promise<MetricsResponse>
}
```

[Full API Reference](API.md)

## CLI Commands

### Database Management
```bash
llm-memory-graph stats              # Show statistics
llm-memory-graph verify             # Verify integrity
llm-memory-graph flush              # Flush to disk
```

### Session Operations
```bash
llm-memory-graph session get <id>   # Get session details
```

### Querying
```bash
llm-memory-graph query -t prompt    # Query prompts
llm-memory-graph query -s <id>      # Query by session
```

### Export/Import
```bash
llm-memory-graph export database -o backup.json
llm-memory-graph import -i backup.json
```

[Full CLI Reference](cli/README.md)

## Development

### Building from Source
```bash
# Clone repository
git clone https://github.com/globalbusinessadvisors/llm-memory-graph.git
cd llm-memory-graph

# Build Rust components
cargo build --release

# Build TypeScript client
cd clients/typescript
npm install
npm run build

# Generate documentation
npm run docs
cargo doc --open
```

### Running Tests
```bash
# Rust tests
cargo test

# TypeScript tests
cd clients/typescript
npm test

# Integration tests
cargo test --test integration_tests
```

[Contributing Guide](../CONTRIBUTING.md)

## Deployment

### Docker
```bash
docker run -p 50051:50051 \
  -v $(pwd)/data:/data \
  ghcr.io/globalbusinessadvisors/llm-memory-graph:latest
```

### Kubernetes
```bash
kubectl apply -f deploy/kubernetes/
```

[Deployment Guide](DEPLOYMENT_GUIDE.md)

## Support

### Documentation
- [Quick Start](guides/quickstart.md)
- [API Reference](API.md)
- [Examples](EXAMPLES.md)

### Community
- [GitHub Issues](https://github.com/globalbusinessadvisors/llm-memory-graph/issues)
- [Discussions](https://github.com/globalbusinessadvisors/llm-memory-graph/discussions)

### Contributing
- [Contributing Guidelines](../CONTRIBUTING.md)
- [Code of Conduct](../CONTRIBUTING.md#code-of-conduct)

## License

This project is licensed under MIT OR Apache-2.0.

See [LICENSE-MIT](../LICENSE-MIT) and [LICENSE-APACHE](../LICENSE-APACHE) for details.

## Acknowledgments

- LLM DevOps Contributors
- Open source community

---

**Need Help?** Check the [Quick Start Guide](guides/quickstart.md) or [open an issue](https://github.com/globalbusinessadvisors/llm-memory-graph/issues).
