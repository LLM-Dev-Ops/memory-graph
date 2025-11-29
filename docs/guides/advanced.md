# Advanced Guide

Advanced features and patterns for LLM Memory Graph.

## Streaming APIs

### Event Streaming

```typescript
await client.streamEvents({
  sessionId: session.id,
  eventTypes: ['NODE_CREATED', 'EDGE_CREATED'],
  onData: (event) => {
    console.log('Event:', event.type, event.payload);
  },
  onError: (error) => {
    console.error('Stream error:', error);
  },
  onEnd: () => {
    console.log('Stream ended');
  }
});
```

### Query Streaming

```typescript
await client.streamQueryResults({
  onData: (node) => {
    processNode(node);
  },
  onError: (error) => {
    handleError(error);
  },
  onEnd: () => {
    finishProcessing();
  }
});
```

## Template System

### Advanced Templates

```typescript
const template = await client.createTemplate({
  template: {
    name: 'code_review',
    templateText: `Review this {{language}} code:
\`\`\`{{language}}
{{code}}
\`\`\`
Focus on: {{focus_areas}}`,
    variables: [
      { name: 'language', required: true },
      { name: 'code', required: true },
      { name: 'focus_areas', required: false, defaultValue: 'security, performance' }
    ],
    version: 1
  }
});
```

### Template Inheritance

```typescript
// Base template
const baseTemplate = await client.createTemplate({
  template: {
    name: 'base_greeting',
    templateText: 'Hello {{name}}!'
  }
});

// Extended template
const extendedTemplate = await client.createTemplate({
  template: {
    name: 'formal_greeting',
    templateText: 'Dear {{title}} {{name}}, {{message}}',
    metadata: { inherits: baseTemplate.id }
  }
});
```

## Multi-Agent Workflows

```typescript
async function multiAgentWorkflow(client: MemoryGraphClient, task: string) {
  const session = await client.createSession({
    metadata: { workflow: 'multi-agent' }
  });

  // Agent 1: Planner
  const plan = await createAgent(client, 'planner');
  const planPrompt = await client.addPrompt({
    sessionId: session.id,
    content: `Plan: ${task}`,
    metadata: { agent: plan.id }
  });

  // Agent 2: Executor
  const executor = await createAgent(client, 'executor');
  // ... agent workflow continues
}
```

## Performance Optimization

### Connection Pooling

```typescript
class ClientPool {
  private clients: MemoryGraphClient[] = [];

  async getClient(): Promise<MemoryGraphClient> {
    if (this.clients.length === 0) {
      return new MemoryGraphClient({ address: 'localhost:50051' });
    }
    return this.clients.pop()!;
  }

  async releaseClient(client: MemoryGraphClient) {
    this.clients.push(client);
  }
}
```

### Batch Operations

```typescript
async function batchAddPrompts(
  client: MemoryGraphClient,
  sessionId: string,
  prompts: string[]
) {
  const promises = prompts.map(content =>
    client.addPrompt({ sessionId, content })
  );
  return Promise.all(promises);
}
```

### Caching Strategy

```typescript
class CachedClient {
  private cache = new Map<string, any>();

  async getNode(nodeId: string): Promise<Node> {
    if (this.cache.has(nodeId)) {
      return this.cache.get(nodeId);
    }
    const node = await this.client.getNode(nodeId);
    this.cache.set(nodeId, node);
    return node;
  }
}
```

## Error Handling Patterns

### Retry with Exponential Backoff

```typescript
async function withRetry<T>(
  operation: () => Promise<T>,
  maxRetries: number = 3
): Promise<T> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await operation();
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      await new Promise(resolve =>
        setTimeout(resolve, Math.pow(2, i) * 100)
      );
    }
  }
  throw new Error('Max retries exceeded');
}
```

### Circuit Breaker

```typescript
class CircuitBreaker {
  private failures = 0;
  private threshold = 5;
  private isOpen = false;

  async execute<T>(operation: () => Promise<T>): Promise<T> {
    if (this.isOpen) {
      throw new Error('Circuit breaker is open');
    }
    try {
      const result = await operation();
      this.failures = 0;
      return result;
    } catch (error) {
      this.failures++;
      if (this.failures >= this.threshold) {
        this.isOpen = true;
      }
      throw error;
    }
  }
}
```

## Monitoring and Metrics

### Custom Metrics

```typescript
const metrics = await client.getMetrics();
console.log({
  totalNodes: metrics.totalNodes,
  totalEdges: metrics.totalEdges,
  avgLatency: metrics.avgWriteLatencyMs,
  requestsPerSecond: metrics.requestsPerSecond
});
```

### Health Monitoring

```typescript
async function monitorHealth(client: MemoryGraphClient) {
  setInterval(async () => {
    try {
      const health = await client.checkHealth();
      if (health.status !== 'SERVING') {
        alertOps('Service degraded');
      }
    } catch (error) {
      alertOps('Health check failed');
    }
  }, 60000);
}
```

## See Also

- [API Documentation](../API.md)
- [Quick Start Guide](quickstart.md)
- [CLI Reference](../cli/README.md)
