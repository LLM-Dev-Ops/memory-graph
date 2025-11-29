# Production-Ready Features

This document describes the enterprise-ready features added to the LLM Memory Graph TypeScript SDK.

## Table of Contents

- [Error Handling](#error-handling)
- [Retry Logic](#retry-logic)
- [Input Validation](#input-validation)
- [Connection Management](#connection-management)
- [Type Safety](#type-safety)
- [Developer Experience](#developer-experience)

## Error Handling

### Comprehensive Error Classes

The SDK provides a complete error hierarchy that maps gRPC status codes to meaningful error types:

```typescript
import {
  MemoryGraphError,
  ConnectionError,
  ValidationError,
  NotFoundError,
  TimeoutError,
  RateLimitError,
  AuthenticationError,
  AuthorizationError,
  InternalServerError,
  isMemoryGraphError,
  isRetryableError
} from '@llm-dev-ops/llm-memory-graph-client';

try {
  const session = await client.getSession('invalid-id');
} catch (error) {
  if (error instanceof NotFoundError) {
    console.log('Session not found:', error.resourceId);
  } else if (error instanceof ValidationError) {
    console.log('Invalid input:', error.field, error.message);
  } else if (isRetryableError(error)) {
    console.log('Temporary error, will retry');
  }
}
```

### Error Types

| Error Class | HTTP Code | Use Case |
|------------|-----------|----------|
| `ValidationError` | 400 | Invalid input parameters |
| `AuthenticationError` | 401 | Authentication failed |
| `AuthorizationError` | 403 | Insufficient permissions |
| `NotFoundError` | 404 | Resource not found |
| `TimeoutError` | 408 | Operation timed out |
| `AlreadyExistsError` | 409 | Resource already exists |
| `RateLimitError` | 429 | Rate limit exceeded |
| `InternalServerError` | 500 | Server error |
| `ConnectionError` | 503 | Connection failed |
| `ServiceUnavailableError` | 503 | Service unavailable |

### Error Properties

All errors extend `MemoryGraphError` and include:

- `code`: String error code for programmatic handling
- `statusCode`: HTTP-style status code
- `details`: Additional error details
- `cause`: Original error that caused this error
- `toJSON()`: Serialize error for logging

## Retry Logic

### Automatic Retry with Exponential Backoff

Configure retry behavior when creating the client:

```typescript
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  retryPolicy: {
    maxRetries: 5,
    initialBackoff: 100,      // Start with 100ms
    maxBackoff: 10000,        // Cap at 10 seconds
    backoffMultiplier: 2,     // Double each time
    useJitter: true,          // Add randomness
    onRetry: (error, attempt, delayMs) => {
      console.log(`Retry ${attempt} after ${delayMs}ms`);
    }
  }
});
```

### Manual Retry

Use the `withRetry` function for custom retry logic:

```typescript
import { withRetry } from '@llm-dev-ops/llm-memory-graph-client';

const result = await withRetry(
  async () => {
    return await client.getNode('node-id');
  },
  {
    maxRetries: 3,
    initialBackoff: 200,
    onRetry: (error, attempt, delay) => {
      logger.warn(`Retrying after ${delay}ms`, { error, attempt });
    }
  }
);
```

### Retry Manager

For advanced retry scenarios with context:

```typescript
import { RetryManager } from '@llm-dev-ops/llm-memory-graph-client';

const retryManager = new RetryManager({
  maxRetries: 5,
  initialBackoff: 100,
  maxBackoff: 5000
});

const result = await retryManager.execute(async (context) => {
  console.log(`Attempt ${context.attempt}`);
  console.log(`Elapsed time: ${context.elapsedMs}ms`);
  console.log(`Previous errors: ${context.errors.length}`);

  return await client.getNode('node-id');
});
```

### Circuit Breaker

Prevent cascading failures with the circuit breaker pattern:

```typescript
import { CircuitBreaker, CircuitState } from '@llm-dev-ops/llm-memory-graph-client';

const breaker = new CircuitBreaker({
  failureThreshold: 5,      // Open after 5 failures
  resetTimeout: 30000,      // Try again after 30s
  successThreshold: 2       // Close after 2 successes
});

try {
  const result = await breaker.execute(() => client.getNode('node-id'));
} catch (error) {
  if (breaker.getState() === CircuitState.OPEN) {
    console.log('Circuit is open, service is unhealthy');
  }
}
```

## Input Validation

### Automatic Validation

All client methods automatically validate inputs:

```typescript
// ValidationError: sessionId cannot be empty
await client.getSession('');

// ValidationError: limit must be between 1 and 10000
await client.listSessions(0);

// ValidationError: tokenUsage.totalTokens must equal promptTokens + completionTokens
await client.addResponse({
  promptId: 'prompt-123',
  content: 'Response',
  tokenUsage: {
    promptTokens: 10,
    completionTokens: 20,
    totalTokens: 25  // Should be 30
  }
});
```

### Manual Validation

Use validators directly for custom validation:

```typescript
import {
  validateSessionId,
  validateNodeId,
  validateNonEmptyString,
  validatePositiveNumber,
  validateMetadata,
  validateTokenUsage
} from '@llm-dev-ops/llm-memory-graph-client';

// Validate individual values
validateSessionId(sessionId);
validatePositiveNumber(temperature, 'temperature');

// Validate complex objects
validateTokenUsage({
  promptTokens: 10,
  completionTokens: 20,
  totalTokens: 30
});
```

### Type Guards

Runtime type checking with assertions:

```typescript
import {
  isPromptNode,
  isResponseNode,
  isToolInvocationNode
} from '@llm-dev-ops/llm-memory-graph-client';

const node = await client.getNode('node-id');

if (isPromptNode(node)) {
  console.log('Prompt:', node.data.content);
}

if (isResponseNode(node)) {
  console.log('Tokens:', node.data.tokenUsage.totalTokens);
}
```

## Connection Management

### Health Monitoring

Monitor connection health with periodic checks:

```typescript
// Start automatic health checks every 30 seconds
client.startHealthChecks(30000);

// Check manually
try {
  const health = await client.health();
  console.log('Status:', health.status);
  console.log('Uptime:', health.uptimeSeconds);
} catch (error) {
  console.log('Service is unhealthy');
}

// Stop health checks
client.stopHealthChecks();
```

### Wait for Ready

Ensure client is connected before operations:

```typescript
const client = new MemoryGraphClient({ address: 'localhost:50051' });

// Wait up to 5 seconds for connection
await client.waitForReady(5000);

// Now safe to use
const session = await client.createSession();
```

### Auto-Reconnection

Client automatically attempts to reconnect on connection loss:

```typescript
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  retryPolicy: { maxRetries: 3 }
});

// Health checks will trigger auto-reconnect
client.startHealthChecks(30000);

// Check connection status
if (client.isConnected()) {
  console.log('Connected');
}

if (client.isClosing()) {
  console.log('Shutting down');
}
```

### Graceful Shutdown

Properly close connections with cleanup:

```typescript
// Async close (recommended)
await client.close(5000);  // 5 second timeout

// Or use in finally block
try {
  await client.createSession();
} finally {
  await client.close();
}
```

### Configuration Access

Get and update client configuration at runtime:

```typescript
// Get current config (read-only)
const config = client.getConfig();
console.log('Address:', config.address);

// Get retry policy
const retryPolicy = client.getRetryPolicy();
console.log('Max retries:', retryPolicy.maxRetries);

// Update retry policy
client.updateRetryPolicy({
  maxRetries: 10,
  initialBackoff: 200
});
```

## Type Safety

### Builder Pattern (Example)

Create complex requests safely:

```typescript
// All fields are validated
const prompt = await client.addPrompt({
  sessionId: session.id,
  content: 'What is TypeScript?',
  metadata: {
    model: 'gpt-4',
    temperature: 0.7,
    maxTokens: 1000,
    toolsAvailable: ['search', 'calculator'],
    custom: {
      source: 'web-ui',
      userId: 'user-123'
    }
  }
});
```

### Type Definitions

All types are exported for use in your code:

```typescript
import type {
  Session,
  Node,
  Edge,
  NodeType,
  EdgeType,
  PromptNode,
  ResponseNode,
  TokenUsage,
  QueryOptions,
  ClientConfig,
  RetryPolicy
} from '@llm-dev-ops/llm-memory-graph-client';

function processNode(node: Node): void {
  if (node.type === NodeType.PROMPT) {
    const data = node.data as PromptNode;
    console.log('Prompt:', data.content);
  }
}
```

## Developer Experience

### Utility Functions

Helpful utilities for common tasks:

```typescript
import {
  sleep,
  debounce,
  throttle,
  formatDuration,
  formatTimestamp,
  calculateTokenUsage,
  batchProcess,
  chunk,
  deepClone,
  deepMerge,
  withTimeout
} from '@llm-dev-ops/llm-memory-graph-client';

// Sleep/delay
await sleep(1000);

// Format duration
const duration = formatDuration(5432);  // "5.4s"

// Batch processing with rate limiting
const results = await batchProcess(
  nodeIds,
  (id) => client.getNode(id),
  10,   // Process 10 at a time
  100   // Wait 100ms between batches
);

// Add timeout to any operation
const result = await withTimeout(
  client.getNode('node-id'),
  5000,
  'Operation timed out'
);

// Calculate total token usage
const total = calculateTokenUsage([
  { promptTokens: 10, completionTokens: 20, totalTokens: 30 },
  { promptTokens: 15, completionTokens: 25, totalTokens: 40 }
]);
console.log('Total tokens:', total.totalTokens);  // 70
```

### JSDoc Documentation

All public APIs have comprehensive JSDoc comments:

```typescript
/**
 * Create a new session
 *
 * @param options - Session creation options
 * @returns Promise with the created session
 * @throws {ValidationError} If options are invalid
 * @throws {MemoryGraphError} If creation fails
 *
 * @example
 * ```typescript
 * const session = await client.createSession({
 *   metadata: { user: 'john', context: 'chat' }
 * });
 * ```
 */
async createSession(options?: CreateSessionOptions): Promise<Session>
```

### Helper Functions

Filter and sort nodes:

```typescript
import {
  filterNodesByType,
  filterEdgesByType,
  sortNodesByTime,
  groupNodesBySession
} from '@llm-dev-ops/llm-memory-graph-client';

const nodes = await client.query({ sessionId: 'session-123' });

// Filter by type
const prompts = filterNodesByType(nodes.nodes, NodeType.PROMPT);

// Sort by time
const newest = sortNodesByTime(nodes.nodes, false);  // Newest first

// Group by session
const grouped = groupNodesBySession(nodes.nodes);
for (const [sessionId, sessionNodes] of grouped.entries()) {
  console.log(`Session ${sessionId}: ${sessionNodes.length} nodes`);
}
```

### Error Handling Best Practices

```typescript
import { isMemoryGraphError, isRetryableError } from '@llm-dev-ops/llm-memory-graph-client';

try {
  await client.getSession('session-id');
} catch (error) {
  // Type-safe error handling
  if (isMemoryGraphError(error)) {
    console.log('Error code:', error.code);
    console.log('Status:', error.statusCode);
    console.log('Details:', error.details);

    // Check if retryable
    if (isRetryableError(error)) {
      console.log('This error can be retried');
    }

    // Log as JSON
    console.log(JSON.stringify(error.toJSON(), null, 2));
  }
}
```

## Complete Example

See `examples/production-features.ts` for a comprehensive demonstration of all features.

```bash
# Run the example
npm run build
node dist/examples/production-features.js
```

## Migration Guide

### From Basic Client

```typescript
// Before
const client = new MemoryGraphClient({
  address: 'localhost:50051'
});

const session = await client.createSession();
client.close();

// After (with production features)
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  retryPolicy: {
    maxRetries: 3,
    initialBackoff: 100
  }
});

try {
  await client.waitForReady(5000);
  client.startHealthChecks(30000);

  const session = await client.createSession();

} catch (error) {
  if (isMemoryGraphError(error)) {
    console.error('Error:', error.code, error.message);
  }
} finally {
  await client.close();
}
```

## Performance Considerations

### Batch Operations

Use batch operations when working with multiple items:

```typescript
// ❌ Inefficient - sequential requests
for (const id of nodeIds) {
  await client.getNode(id);
}

// ✅ Better - batch request
const nodes = await client.batchGetNodes(nodeIds);

// ✅ Best - batch with rate limiting
const nodes = await batchProcess(
  nodeIds,
  (id) => client.getNode(id),
  10,  // Concurrency
  100  // Delay
);
```

### Connection Pooling

The client maintains a single gRPC connection with built-in multiplexing. For high-throughput scenarios, consider creating multiple client instances.

### Retry Configuration

Tune retry settings based on your use case:

```typescript
// High availability - aggressive retries
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  retryPolicy: {
    maxRetries: 10,
    initialBackoff: 50,
    maxBackoff: 5000
  }
});

// Latency sensitive - fewer retries
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  retryPolicy: {
    maxRetries: 2,
    initialBackoff: 100,
    maxBackoff: 1000
  }
});
```

## Testing

### Mock Error Scenarios

```typescript
import { ValidationError, NotFoundError } from '@llm-dev-ops/llm-memory-graph-client';

// Test error handling
it('should handle not found errors', async () => {
  try {
    await client.getSession('non-existent');
    fail('Should have thrown');
  } catch (error) {
    expect(error).toBeInstanceOf(NotFoundError);
  }
});
```

## Troubleshooting

### Common Issues

**Connection Refused**
```typescript
// Use waitForReady with health checks
await client.waitForReady(5000);
client.startHealthChecks(30000);
```

**Timeout Errors**
```typescript
// Increase timeout
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  timeout: 60000  // 60 seconds
});
```

**Validation Errors**
```typescript
// Check error details
catch (error) {
  if (error instanceof ValidationError) {
    console.log('Field:', error.field);
    console.log('Details:', error.details);
  }
}
```

## Best Practices

1. **Always close the client**: Use `await client.close()` when done
2. **Use health checks**: Monitor connection health in production
3. **Configure retries**: Set appropriate retry policy for your use case
4. **Handle errors**: Use type-safe error handling with error classes
5. **Validate inputs**: Use provided validators for custom validation
6. **Batch operations**: Process multiple items efficiently
7. **Monitor metrics**: Use `client.getMetrics()` for observability
8. **Graceful shutdown**: Wait for operations to complete before closing

## Next Steps

- Review the [API Documentation](./README.md)
- Explore [Examples](./examples/)
- Check [Type Definitions](./src/types.ts)
- Read [Error Handling Guide](./src/errors.ts)
