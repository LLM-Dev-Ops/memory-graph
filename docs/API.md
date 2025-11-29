# LLM Memory Graph API Documentation

Complete API reference for all LLM Memory Graph SDKs and interfaces.

## Table of Contents

- [Overview](#overview)
- [TypeScript/JavaScript Client](#typescriptjavascript-client)
- [Rust Client](#rust-client)
- [gRPC API](#grpc-api)
- [REST API](#rest-api)
- [Data Models](#data-models)
- [Error Handling](#error-handling)

## Overview

LLM Memory Graph provides multiple interfaces for interaction:

1. **TypeScript/JavaScript Client** - High-level gRPC client for Node.js applications
2. **Rust Client** - Native Rust client library
3. **gRPC API** - Protocol Buffers-based RPC interface
4. **CLI** - Command-line management tool

### Quick Links

- [TypeDoc Documentation](typescript/index.html) - Full TypeScript API reference
- [Rustdoc Documentation](rust/llm_memory_graph/index.html) - Full Rust API reference
- [CLI Reference](cli/README.md) - Command-line interface documentation

## TypeScript/JavaScript Client

### Installation

```bash
npm install @llm-dev-ops/llm-memory-graph-client
```

### Basic Usage

```typescript
import { MemoryGraphClient } from '@llm-dev-ops/llm-memory-graph-client';

// Create client
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  useTls: false
});

// Create session
const session = await client.createSession({
  metadata: { user: 'john' }
});

// Add prompt
const prompt = await client.addPrompt({
  sessionId: session.id,
  content: 'What is the weather?',
  metadata: { model: 'gpt-4' }
});

// Add response
const response = await client.addResponse({
  promptId: prompt.id,
  content: 'The weather is sunny.',
  tokenUsage: {
    promptTokens: 10,
    completionTokens: 8,
    totalTokens: 18
  }
});

// Close client
await client.close();
```

### Client Configuration

```typescript
interface ClientConfig {
  // Server address (host:port or just host)
  address: string;

  // Optional port (overrides address port)
  port?: number;

  // Enable TLS
  useTls?: boolean;

  // TLS options
  tlsOptions?: {
    rootCerts?: Buffer;
    privateKey?: Buffer;
    certChain?: Buffer;
  };

  // Authentication credentials
  credentials?: {
    username: string;
    password: string;
  };

  // Request timeout in milliseconds
  timeout?: number;

  // Retry policy
  retryPolicy?: {
    maxRetries: number;
    initialBackoff: number;
    maxBackoff: number;
    backoffMultiplier: number;
  };
}
```

### Core Methods

#### Session Management

```typescript
// Create session
createSession(options?: CreateSessionOptions): Promise<Session>

// Get session
getSession(sessionId: string): Promise<Session>

// Close session
closeSession(sessionId: string): Promise<void>
```

#### Node Operations

```typescript
// Add prompt
addPrompt(request: AddPromptRequest): Promise<PromptNode>

// Add response
addResponse(request: AddResponseRequest): Promise<ResponseNode>

// Add tool invocation
addToolInvocation(request: AddToolInvocationRequest): Promise<ToolInvocationNode>

// Get node
getNode(nodeId: string): Promise<Node>

// Get nodes
getNodes(nodeIds: string[]): Promise<Node[]>
```

#### Querying

```typescript
// Query nodes
queryNodes(options: QueryOptions): Promise<QueryResult>

// Get session nodes
getSessionNodes(sessionId: string, limit?: number, offset?: number): Promise<Node[]>

// Get edges
getEdges(nodeId: string, direction?: EdgeDirection, edgeType?: EdgeType): Promise<Edge[]>
```

#### Templates

```typescript
// Create template
createTemplate(request: CreateTemplateRequest): Promise<TemplateNode>

// Get template
getTemplate(templateId: string): Promise<TemplateNode>

// Instantiate template
instantiateTemplate(request: InstantiateTemplateRequest): Promise<PromptNode>
```

#### Streaming

```typescript
// Stream query results
streamQueryResults(options: StreamOptions): Promise<void>

// Stream events
streamEvents(options: EventStreamOptions): Promise<void>

// Stream session events
streamSessionEvents(options: SessionEventStreamOptions): Promise<void>
```

#### Health & Metrics

```typescript
// Check health
checkHealth(): Promise<HealthResponse>

// Get metrics
getMetrics(): Promise<MetricsResponse>
```

### Error Handling

```typescript
import {
  MemoryGraphError,
  ConnectionError,
  ValidationError,
  NotFoundError,
  AlreadyExistsError,
  TimeoutError
} from '@llm-dev-ops/llm-memory-graph-client';

try {
  await client.addPrompt({ sessionId, content });
} catch (error) {
  if (error instanceof ValidationError) {
    console.error('Invalid input:', error.message);
  } else if (error instanceof NotFoundError) {
    console.error('Resource not found:', error.message);
  } else if (error instanceof ConnectionError) {
    console.error('Connection failed:', error.message);
  }
}
```

### Retry Policy

```typescript
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  retryPolicy: {
    maxRetries: 3,
    initialBackoff: 100,    // ms
    maxBackoff: 5000,       // ms
    backoffMultiplier: 2.0
  }
});
```

### Complete TypeScript Example

```typescript
import { MemoryGraphClient } from '@llm-dev-ops/llm-memory-graph-client';

async function main() {
  const client = new MemoryGraphClient({
    address: 'localhost:50051',
    useTls: false,
    retryPolicy: {
      maxRetries: 3,
      initialBackoff: 100,
      maxBackoff: 5000
    }
  });

  try {
    // Create session
    const session = await client.createSession({
      metadata: {
        user: 'alice',
        application: 'chatbot'
      }
    });

    console.log('Session created:', session.id);

    // Add multiple prompts and responses
    for (let i = 0; i < 3; i++) {
      const prompt = await client.addPrompt({
        sessionId: session.id,
        content: `Question ${i + 1}`,
        metadata: {
          model: 'gpt-4',
          temperature: 0.7
        }
      });

      const response = await client.addResponse({
        promptId: prompt.id,
        content: `Answer ${i + 1}`,
        tokenUsage: {
          promptTokens: 10,
          completionTokens: 15,
          totalTokens: 25
        },
        metadata: {
          model: 'gpt-4',
          finishReason: 'stop',
          latencyMs: 850
        }
      });

      console.log(`Pair ${i + 1}: ${prompt.id} -> ${response.id}`);
    }

    // Query session nodes
    const nodes = await client.getSessionNodes(session.id);
    console.log(`Total nodes in session: ${nodes.length}`);

    // Query with filters
    const prompts = await client.queryNodes({
      sessionId: session.id,
      nodeType: 'prompt',
      limit: 10
    });

    console.log(`Prompts found: ${prompts.nodes.length}`);

    // Stream events
    await client.streamSessionEvents({
      sessionId: session.id,
      onData: (event) => {
        console.log('Event:', event.type);
      },
      onError: (error) => {
        console.error('Stream error:', error);
      },
      onEnd: () => {
        console.log('Stream ended');
      }
    });

    // Health check
    const health = await client.checkHealth();
    console.log('Server status:', health.status);

    // Metrics
    const metrics = await client.getMetrics();
    console.log('Total nodes:', metrics.totalNodes);

  } finally {
    await client.close();
  }
}

main().catch(console.error);
```

## Rust Client

### Installation

Add to `Cargo.toml`:

```toml
[dependencies]
llm-memory-graph-client = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use llm_memory_graph_client::{Client, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let config = ClientConfig {
        endpoint: "http://localhost:50051".to_string(),
        timeout: std::time::Duration::from_secs(30),
        ..Default::default()
    };

    let mut client = Client::new(config).await?;

    // Create session
    let session = client.create_session(None).await?;
    println!("Session ID: {}", session.id);

    // Add prompt
    let prompt = client.add_prompt(
        &session.id,
        "What is the weather?",
        None
    ).await?;

    // Add response
    let response = client.add_response(
        &prompt.id,
        "The weather is sunny.",
        Some(TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 8,
            total_tokens: 18,
        }),
        None
    ).await?;

    println!("Response ID: {}", response.id);

    Ok(())
}
```

### Core Types

```rust
pub struct Client {
    // gRPC client implementation
}

pub struct Session {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub metadata: HashMap<String, String>,
}

pub struct PromptNode {
    pub id: String,
    pub session_id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<HashMap<String, String>>,
}

pub struct ResponseNode {
    pub id: String,
    pub prompt_id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub token_usage: Option<TokenUsage>,
    pub metadata: Option<HashMap<String, String>>,
}
```

### Methods

```rust
impl Client {
    // Create new client
    pub async fn new(config: ClientConfig) -> Result<Self>;

    // Session management
    pub async fn create_session(&mut self, metadata: Option<HashMap<String, String>>) -> Result<Session>;
    pub async fn get_session(&mut self, session_id: &str) -> Result<Session>;
    pub async fn close_session(&mut self, session_id: &str) -> Result<()>;

    // Node operations
    pub async fn add_prompt(&mut self, session_id: &str, content: &str, metadata: Option<HashMap<String, String>>) -> Result<PromptNode>;
    pub async fn add_response(&mut self, prompt_id: &str, content: &str, token_usage: Option<TokenUsage>, metadata: Option<HashMap<String, String>>) -> Result<ResponseNode>;
    pub async fn get_node(&mut self, node_id: &str) -> Result<Node>;

    // Querying
    pub async fn query_nodes(&mut self, options: QueryOptions) -> Result<Vec<Node>>;
    pub async fn get_session_nodes(&mut self, session_id: &str, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Node>>;

    // Health
    pub async fn check_health(&mut self) -> Result<HealthStatus>;
    pub async fn get_metrics(&mut self) -> Result<Metrics>;
}
```

## gRPC API

### Protocol Buffers Definition

The gRPC API is defined in `proto/memory_graph.proto`. Key services:

```protobuf
service MemoryGraph {
  // Session management
  rpc CreateSession(CreateSessionRequest) returns (Session);
  rpc GetSession(GetSessionRequest) returns (Session);
  rpc CloseSession(CloseSessionRequest) returns (CloseSessionResponse);

  // Node operations
  rpc AddPrompt(AddPromptRequest) returns (PromptNode);
  rpc AddResponse(AddResponseRequest) returns (ResponseNode);
  rpc AddToolInvocation(AddToolInvocationRequest) returns (ToolInvocationNode);
  rpc GetNode(GetNodeRequest) returns (Node);
  rpc GetNodes(GetNodesRequest) returns (GetNodesResponse);

  // Querying
  rpc QueryNodes(QueryNodesRequest) returns (QueryNodesResponse);
  rpc GetSessionNodes(GetSessionNodesRequest) returns (GetSessionNodesResponse);
  rpc GetEdges(GetEdgesRequest) returns (GetEdgesResponse);

  // Templates
  rpc CreateTemplate(CreateTemplateRequest) returns (TemplateNode);
  rpc GetTemplate(GetTemplateRequest) returns (TemplateNode);
  rpc InstantiateTemplate(InstantiateTemplateRequest) returns (PromptNode);

  // Streaming
  rpc StreamQueryResults(QueryNodesRequest) returns (stream Node);
  rpc StreamEvents(StreamEventsRequest) returns (stream Event);
  rpc StreamSessionEvents(StreamSessionEventsRequest) returns (stream SessionEvent);

  // Health & Metrics
  rpc CheckHealth(HealthRequest) returns (HealthResponse);
  rpc GetMetrics(MetricsRequest) returns (MetricsResponse);
}
```

### Connection

```bash
# Default endpoint
grpcurl -plaintext localhost:50051 list

# With TLS
grpcurl -cacert ca.crt server.example.com:50051 list
```

### Example Requests

```bash
# Create session
grpcurl -plaintext -d '{"metadata": {"user": "alice"}}' \
  localhost:50051 memory_graph.MemoryGraph/CreateSession

# Add prompt
grpcurl -plaintext -d '{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "content": "Hello, world!",
  "metadata": {"model": "gpt-4"}
}' localhost:50051 memory_graph.MemoryGraph/AddPrompt

# Query nodes
grpcurl -plaintext -d '{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "limit": 10
}' localhost:50051 memory_graph.MemoryGraph/QueryNodes
```

## Data Models

### Session

Represents a conversation or interaction session.

```typescript
interface Session {
  id: string;              // UUID
  createdAt: Date;         // Creation timestamp
  updatedAt: Date;         // Last update timestamp
  isActive: boolean;       // Active status
  metadata: Record<string, string>;  // Custom metadata
}
```

### Node Types

#### Prompt Node

```typescript
interface PromptNode {
  id: string;
  sessionId: string;
  content: string;
  timestamp: Date;
  metadata?: PromptMetadata;
}

interface PromptMetadata {
  model?: string;
  temperature?: number;
  maxTokens?: number;
  toolsAvailable?: string[];
  custom?: Record<string, any>;
}
```

#### Response Node

```typescript
interface ResponseNode {
  id: string;
  promptId: string;
  content: string;
  timestamp: Date;
  tokenUsage?: TokenUsage;
  metadata?: ResponseMetadata;
}

interface TokenUsage {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
}

interface ResponseMetadata {
  model?: string;
  finishReason?: string;
  latencyMs?: number;
  custom?: Record<string, any>;
}
```

#### Tool Invocation Node

```typescript
interface ToolInvocationNode {
  id: string;
  responseId: string;
  toolName: string;
  parameters: Record<string, any>;
  status: 'pending' | 'running' | 'success' | 'failed';
  result?: any;
  error?: string;
  durationMs?: number;
  retryCount?: number;
  timestamp: Date;
  metadata?: Record<string, any>;
}
```

#### Template Node

```typescript
interface TemplateNode {
  id: string;
  name: string;
  templateText: string;
  variables: VariableSpec[];
  version: number;
  usageCount: number;
  createdAt: Date;
  metadata?: Record<string, any>;
}

interface VariableSpec {
  name: string;
  typeHint?: string;
  required: boolean;
  defaultValue?: string;
  validationPattern?: string;
  description?: string;
}
```

#### Agent Node

```typescript
interface AgentNode {
  id: string;
  name: string;
  role: string;
  capabilities: string[];
  status: 'active' | 'inactive' | 'paused';
  createdAt: Date;
  metadata?: Record<string, any>;
}
```

### Edge Types

```typescript
enum EdgeType {
  BELONGS_TO = 'BELONGS_TO',           // Node belongs to session
  RESPONDS_TO = 'RESPONDS_TO',         // Response to prompt
  FOLLOWS = 'FOLLOWS',                 // Sequential relationship
  INVOKES = 'INVOKES',                 // Tool invocation
  HANDLED_BY = 'HANDLED_BY',           // Agent handling
  INSTANTIATES = 'INSTANTIATES',       // Template instantiation
  INHERITS = 'INHERITS',               // Template inheritance
  TRANSFERS_TO = 'TRANSFERS_TO',       // Agent transfer
  REFERENCES = 'REFERENCES'            // General reference
}

interface Edge {
  id: string;
  fromNodeId: string;
  toNodeId: string;
  type: EdgeType;
  createdAt: Date;
  properties?: Record<string, any>;
}
```

### Query Options

```typescript
interface QueryOptions {
  sessionId?: string;
  nodeType?: NodeType;
  after?: Date;
  before?: Date;
  limit?: number;
  offset?: number;
  filters?: Record<string, any>;
}

interface QueryResult {
  nodes: Node[];
  totalCount: number;
}
```

## Error Handling

### Error Types

```typescript
class MemoryGraphError extends Error {
  code: string;
  details?: any;
}

class ConnectionError extends MemoryGraphError {}
class ValidationError extends MemoryGraphError {}
class NotFoundError extends MemoryGraphError {}
class AlreadyExistsError extends MemoryGraphError {}
class TimeoutError extends MemoryGraphError {}
class PermissionDeniedError extends MemoryGraphError {}
class InternalError extends MemoryGraphError {}
```

### Error Codes

| Code | Description |
|------|-------------|
| `CANCELLED` | Operation was cancelled |
| `UNKNOWN` | Unknown error |
| `INVALID_ARGUMENT` | Invalid argument provided |
| `DEADLINE_EXCEEDED` | Request timeout |
| `NOT_FOUND` | Resource not found |
| `ALREADY_EXISTS` | Resource already exists |
| `PERMISSION_DENIED` | Permission denied |
| `RESOURCE_EXHAUSTED` | Resource exhausted |
| `FAILED_PRECONDITION` | Failed precondition |
| `ABORTED` | Operation aborted |
| `OUT_OF_RANGE` | Out of range |
| `UNIMPLEMENTED` | Not implemented |
| `INTERNAL` | Internal error |
| `UNAVAILABLE` | Service unavailable |
| `DATA_LOSS` | Data loss |
| `UNAUTHENTICATED` | Authentication required |

### Error Handling Best Practices

```typescript
import { MemoryGraphClient, ValidationError, NotFoundError } from '@llm-dev-ops/llm-memory-graph-client';

async function handleOperation() {
  const client = new MemoryGraphClient({
    address: 'localhost:50051',
    retryPolicy: {
      maxRetries: 3,
      initialBackoff: 100,
      maxBackoff: 5000
    }
  });

  try {
    const result = await client.addPrompt({
      sessionId: 'invalid-id',
      content: 'Test'
    });
  } catch (error) {
    if (error instanceof ValidationError) {
      // Handle validation errors
      console.error('Validation failed:', error.message);
      // Show user-friendly message
    } else if (error instanceof NotFoundError) {
      // Handle not found errors
      console.error('Resource not found:', error.message);
      // Create resource or show error
    } else if (error instanceof ConnectionError) {
      // Handle connection errors
      console.error('Connection failed:', error.message);
      // Retry or show offline message
    } else {
      // Handle unexpected errors
      console.error('Unexpected error:', error);
      // Log and show generic error
    }
  } finally {
    await client.close();
  }
}
```

## See Also

- [TypeScript API Documentation](typescript/index.html)
- [Rust API Documentation](rust/llm_memory_graph/index.html)
- [CLI Reference](cli/README.md)
- [User Guides](guides/quickstart.md)
- [Examples](EXAMPLES.md)
