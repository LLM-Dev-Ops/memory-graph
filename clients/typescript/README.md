# LLM Memory Graph Client

A professional TypeScript/JavaScript client for the LLM-Memory-Graph gRPC service. This package provides a high-level, type-safe API for interacting with the LLM Memory Graph, enabling context tracking, prompt lineage management, and graph-based memory operations for LLM applications.

## Features

- **Full Type Safety**: Complete TypeScript type definitions for all operations
- **Promise-based API**: Modern async/await interface for all RPC calls
- **Streaming Support**: Built-in support for streaming queries and event subscriptions
- **Connection Management**: Automatic connection handling with TLS support
- **Error Handling**: Comprehensive error handling with meaningful error messages
- **Zero Dependencies**: Minimal dependencies using standard gRPC libraries

## Installation

```bash
npm install @llm-dev-ops/llm-memory-graph-client
```

Or with yarn:

```bash
yarn add @llm-dev-ops/llm-memory-graph-client
```

## Quick Start

```typescript
import { MemoryGraphClient, NodeType } from '@llm-dev-ops/llm-memory-graph-client';

// Create a client
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  useTls: false
});

async function example() {
  // Create a session
  const session = await client.createSession({
    metadata: { user: 'john', context: 'chat' }
  });
  console.log('Session created:', session.id);

  // Add a prompt
  const prompt = await client.addPrompt({
    sessionId: session.id,
    content: 'What is the capital of France?',
    metadata: {
      model: 'gpt-4',
      temperature: 0.7,
      toolsAvailable: [],
      custom: {}
    }
  });

  // Add a response
  const response = await client.addResponse({
    promptId: prompt.id,
    content: 'The capital of France is Paris.',
    tokenUsage: {
      promptTokens: 15,
      completionTokens: 8,
      totalTokens: 23
    },
    metadata: {
      model: 'gpt-4',
      finishReason: 'stop',
      latencyMs: 1234,
      custom: {}
    }
  });

  // Query nodes
  const results = await client.query({
    sessionId: session.id,
    nodeType: NodeType.PROMPT,
    limit: 10
  });
  console.log('Found', results.totalCount, 'prompts');

  // Clean up
  client.close();
}

example().catch(console.error);
```

## Configuration

### Basic Configuration

```typescript
const client = new MemoryGraphClient({
  address: 'localhost:50051'
});
```

### TLS Configuration

```typescript
import * as fs from 'fs';

const client = new MemoryGraphClient({
  address: 'example.com:443',
  useTls: true,
  tlsOptions: {
    rootCerts: fs.readFileSync('./ca.pem'),
    privateKey: fs.readFileSync('./key.pem'),
    certChain: fs.readFileSync('./cert.pem')
  }
});
```

### Advanced Configuration

```typescript
const client = new MemoryGraphClient({
  address: 'localhost',
  port: 50051,
  useTls: false,
  timeout: 30000, // 30 seconds
  retryPolicy: {
    maxRetries: 3,
    initialBackoff: 1000,
    maxBackoff: 10000,
    backoffMultiplier: 2
  }
});
```

## API Reference

### Session Management

#### createSession(options?)

Create a new session for organizing related prompts and responses.

```typescript
const session = await client.createSession({
  metadata: {
    user: 'alice',
    application: 'chatbot',
    environment: 'production'
  }
});
```

#### getSession(sessionId)

Retrieve a session by its ID.

```typescript
const session = await client.getSession('session-123');
```

#### deleteSession(sessionId)

Delete a session and all associated data.

```typescript
await client.deleteSession('session-123');
```

#### listSessions(limit?, offset?)

List all sessions with pagination.

```typescript
const { sessions, totalCount } = await client.listSessions(50, 0);
```

### Prompt & Response Operations

#### addPrompt(request)

Add a prompt to a session.

```typescript
const prompt = await client.addPrompt({
  sessionId: 'session-123',
  content: 'Explain quantum computing',
  metadata: {
    model: 'gpt-4',
    temperature: 0.7,
    maxTokens: 2000,
    toolsAvailable: ['search', 'calculator'],
    custom: { priority: 'high' }
  }
});
```

#### addResponse(request)

Add a response to a prompt.

```typescript
const response = await client.addResponse({
  promptId: prompt.id,
  content: 'Quantum computing uses quantum mechanics...',
  tokenUsage: {
    promptTokens: 10,
    completionTokens: 150,
    totalTokens: 160
  },
  metadata: {
    model: 'gpt-4',
    finishReason: 'stop',
    latencyMs: 2500,
    custom: {}
  }
});
```

#### addToolInvocation(request)

Track tool/function calls made during response generation.

```typescript
const toolInvocation = await client.addToolInvocation({
  toolInvocation: {
    id: 'tool-inv-123',
    responseId: response.id,
    toolName: 'search',
    parameters: JSON.stringify({ query: 'quantum computing' }),
    status: 'success',
    result: JSON.stringify({ results: [...] }),
    durationMs: 250,
    retryCount: 0,
    timestamp: new Date(),
    metadata: {}
  }
});
```

### Query Operations

#### query(options)

Query nodes with flexible filtering options.

```typescript
const results = await client.query({
  sessionId: 'session-123',
  nodeType: NodeType.RESPONSE,
  after: new Date('2024-01-01'),
  before: new Date('2024-12-31'),
  limit: 100,
  offset: 0,
  filters: { model: 'gpt-4' }
});

console.log(`Found ${results.totalCount} responses`);
results.nodes.forEach(node => {
  console.log(node.id, node.createdAt);
});
```

#### streamQuery(options, streamOptions)

Stream query results for large result sets.

```typescript
client.streamQuery(
  {
    sessionId: 'session-123',
    nodeType: NodeType.PROMPT
  },
  {
    onData: (node) => {
      console.log('Received node:', node.id);
    },
    onError: (error) => {
      console.error('Stream error:', error);
    },
    onEnd: () => {
      console.log('Stream completed');
    }
  }
);
```

### Node Operations

#### createNode(node)

Create a custom node.

```typescript
const node = await client.createNode({
  id: 'node-123',
  type: NodeType.AGENT,
  createdAt: new Date(),
  data: {
    id: 'agent-1',
    name: 'Assistant',
    role: 'helper',
    capabilities: ['search', 'summarize'],
    status: 'active',
    createdAt: new Date(),
    metadata: {}
  }
});
```

#### getNode(nodeId)

Retrieve a node by ID.

```typescript
const node = await client.getNode('node-123');
```

#### updateNode(node)

Update an existing node.

```typescript
const updatedNode = await client.updateNode({
  ...node,
  data: { ...node.data, status: 'inactive' }
});
```

#### deleteNode(nodeId)

Delete a node.

```typescript
await client.deleteNode('node-123');
```

#### batchCreateNodes(nodes)

Create multiple nodes in a single request.

```typescript
const { nodes, createdCount } = await client.batchCreateNodes([
  node1,
  node2,
  node3
]);
```

#### batchGetNodes(nodeIds)

Retrieve multiple nodes by ID.

```typescript
const nodes = await client.batchGetNodes([
  'node-1',
  'node-2',
  'node-3'
]);
```

### Edge Operations

#### createEdge(edge)

Create a relationship between nodes.

```typescript
const edge = await client.createEdge({
  id: 'edge-123',
  fromNodeId: 'prompt-1',
  toNodeId: 'response-1',
  type: EdgeType.RESPONDS_TO,
  createdAt: new Date(),
  properties: {
    weight: '1.0',
    confidence: '0.95'
  }
});
```

#### getEdges(nodeId, direction?, type?)

Get edges connected to a node.

```typescript
import { EdgeDirection, EdgeType } from '@llm-dev-ops/llm-memory-graph-client';

const edges = await client.getEdges(
  'node-123',
  EdgeDirection.OUTGOING,
  EdgeType.RESPONDS_TO
);
```

#### deleteEdge(edgeId)

Delete an edge.

```typescript
await client.deleteEdge('edge-123');
```

### Template Operations

#### createTemplate(request)

Create a reusable prompt template.

```typescript
const template = await client.createTemplate({
  template: {
    id: 'template-1',
    name: 'greeting',
    templateText: 'Hello {{name}}, welcome to {{place}}!',
    variables: [
      {
        name: 'name',
        typeHint: 'string',
        required: true,
        description: 'User name'
      },
      {
        name: 'place',
        typeHint: 'string',
        required: true,
        description: 'Location'
      }
    ],
    version: '1.0.0',
    usageCount: 0,
    createdAt: new Date(),
    metadata: {}
  }
});
```

#### instantiateTemplate(request)

Create a prompt from a template.

```typescript
const prompt = await client.instantiateTemplate({
  templateId: 'template-1',
  variableValues: {
    name: 'Alice',
    place: 'Wonderland'
  },
  sessionId: 'session-123'
});
```

### Streaming Operations

#### streamEvents(options)

Stream real-time events.

```typescript
import { EventType } from '@llm-dev-ops/llm-memory-graph-client';

client.streamEvents({
  sessionId: 'session-123',
  eventTypes: [EventType.NODE_CREATED, EventType.EDGE_CREATED],
  onData: (event) => {
    console.log('Event:', event.type, event.payload);
  },
  onError: (error) => console.error(error),
  onEnd: () => console.log('Stream ended')
});
```

#### subscribeToSession(options)

Subscribe to all events for a specific session.

```typescript
client.subscribeToSession({
  sessionId: 'session-123',
  onData: (sessionEvent) => {
    console.log('Session event:', sessionEvent.event.type);
  },
  onError: (error) => console.error(error),
  onEnd: () => console.log('Subscription ended')
});
```

### Health & Metrics

#### health()

Check service health.

```typescript
const health = await client.health();
console.log('Status:', health.status);
console.log('Version:', health.version);
console.log('Uptime:', health.uptimeSeconds, 'seconds');
```

#### getMetrics()

Get service metrics.

```typescript
const metrics = await client.getMetrics();
console.log('Total Nodes:', metrics.totalNodes);
console.log('Total Edges:', metrics.totalEdges);
console.log('Active Sessions:', metrics.activeSessions);
console.log('Avg Write Latency:', metrics.avgWriteLatencyMs, 'ms');
console.log('Requests/sec:', metrics.requestsPerSecond);
```

## Type Definitions

The package exports comprehensive TypeScript types for all operations:

```typescript
import {
  // Client
  MemoryGraphClient,
  ClientConfig,

  // Core types
  Session,
  Node,
  Edge,
  NodeType,
  EdgeType,
  EdgeDirection,

  // Node types
  PromptNode,
  ResponseNode,
  ToolInvocationNode,
  AgentNode,
  TemplateNode,

  // Metadata types
  TokenUsage,
  PromptMetadata,
  ResponseMetadata,
  VariableSpec,

  // Request types
  AddPromptRequest,
  AddResponseRequest,
  AddToolInvocationRequest,
  CreateTemplateRequest,
  InstantiateTemplateRequest,

  // Query types
  QueryOptions,
  QueryResult,

  // Event types
  Event,
  EventType,
  SessionEvent,

  // Health & Metrics
  HealthResponse,
  MetricsResponse,
  ServingStatus,

  // Stream types
  StreamOptions,
  EventStreamOptions,
  SessionEventStreamOptions
} from '@llm-dev-ops/llm-memory-graph-client';
```

## Error Handling

All methods return Promises and can throw errors. Use try-catch for error handling:

```typescript
try {
  const session = await client.createSession();
  const prompt = await client.addPrompt({
    sessionId: session.id,
    content: 'Hello, world!'
  });
} catch (error) {
  if (error.code === grpc.status.UNAVAILABLE) {
    console.error('Service is unavailable');
  } else if (error.code === grpc.status.NOT_FOUND) {
    console.error('Resource not found');
  } else {
    console.error('Error:', error.message);
  }
}
```

## Examples

See the [examples](./examples) directory for complete working examples:

- `quickstart.ts` - Basic usage example
- Advanced query patterns
- Streaming operations
- Template usage
- Multi-agent coordination

## Best Practices

1. **Connection Management**: Reuse client instances instead of creating new ones for each request
2. **Error Handling**: Always wrap client calls in try-catch blocks
3. **Streaming**: Use streaming for large result sets to reduce memory usage
4. **Session Organization**: Use meaningful metadata to organize sessions
5. **Resource Cleanup**: Call `client.close()` when done to free resources

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/globalbusinessadvisors/llm-memory-graph.git
cd llm-memory-graph/clients/typescript

# Install dependencies
npm install

# Generate protobuf code
npm run generate

# Build the package
npm run build

# Run tests
npm test
```

### Publishing

```bash
# Build and test
npm run build
npm test

# Publish to npm
npm publish
```

## License

This package is licensed under MIT OR Apache-2.0.

## Support

- GitHub Issues: https://github.com/globalbusinessadvisors/llm-memory-graph/issues
- Documentation: https://github.com/globalbusinessadvisors/llm-memory-graph
- Repository: https://github.com/globalbusinessadvisors/llm-memory-graph

## Contributing

Contributions are welcome! Please see the main repository for contribution guidelines.

---

**LLM Memory Graph Client** - Professional TypeScript/JavaScript client for LLM context tracking and prompt lineage management.
