# Quick Reference - Production Features

Quick reference guide for the most commonly used production features in the LLM Memory Graph TypeScript SDK.

## Table of Contents
- [Error Handling](#error-handling)
- [Retry Configuration](#retry-configuration)
- [Connection Management](#connection-management)
- [Validation](#validation)
- [Batch Operations](#batch-operations)
- [Utilities](#utilities)

## Error Handling

### Basic Error Handling
```typescript
import {
  MemoryGraphClient,
  ValidationError,
  NotFoundError,
  ConnectionError
} from '@llm-dev-ops/llm-memory-graph-client';

try {
  const session = await client.getSession('session-id');
} catch (error) {
  if (error instanceof ValidationError) {
    console.error('Invalid input:', error.field, error.message);
  } else if (error instanceof NotFoundError) {
    console.error('Not found:', error.resourceId);
  } else if (error instanceof ConnectionError) {
    console.error('Connection failed:', error.message);
  }
}
```

### Type-Safe Error Handling
```typescript
import { isMemoryGraphError, isRetryableError } from '@llm-dev-ops/llm-memory-graph-client';

try {
  await client.createSession();
} catch (error) {
  if (isMemoryGraphError(error)) {
    console.log('Error code:', error.code);
    console.log('Status:', error.statusCode);
    console.log('Can retry:', isRetryableError(error));
  }
}
```

### All Error Types
```typescript
import {
  ValidationError,        // 400 - Invalid input
  AuthenticationError,   // 401 - Auth failed
  AuthorizationError,    // 403 - Permission denied
  NotFoundError,         // 404 - Not found
  TimeoutError,          // 408 - Timeout
  AlreadyExistsError,    // 409 - Conflict
  RateLimitError,        // 429 - Rate limit
  InternalServerError,   // 500 - Server error
  ConnectionError,       // 503 - Connection failed
  ServiceUnavailableError // 503 - Service down
} from '@llm-dev-ops/llm-memory-graph-client';
```

## Retry Configuration

### Client-Level Retry
```typescript
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  retryPolicy: {
    maxRetries: 5,
    initialBackoff: 100,    // 100ms
    maxBackoff: 10000,      // 10s max
    backoffMultiplier: 2,   // 2x each time
    useJitter: true,        // Add randomness
    onRetry: (error, attempt, delayMs) => {
      console.log(`Retry ${attempt} after ${delayMs}ms`);
    }
  }
});
```

### Manual Retry
```typescript
import { withRetry } from '@llm-dev-ops/llm-memory-graph-client';

const result = await withRetry(
  () => client.getNode('node-id'),
  {
    maxRetries: 3,
    initialBackoff: 200,
    onRetry: (error, attempt, delay) => {
      logger.warn(`Retrying (${attempt})...`);
    }
  }
);
```

### Retry Manager
```typescript
import { RetryManager } from '@llm-dev-ops/llm-memory-graph-client';

const retryManager = new RetryManager({
  maxRetries: 5,
  initialBackoff: 100
});

const result = await retryManager.execute(async (context) => {
  console.log(`Attempt ${context.attempt}`);
  return await client.getNode('node-id');
});
```

### Circuit Breaker
```typescript
import { CircuitBreaker } from '@llm-dev-ops/llm-memory-graph-client';

const breaker = new CircuitBreaker({
  failureThreshold: 5,    // Open after 5 failures
  resetTimeout: 30000,    // Try again after 30s
  successThreshold: 2     // Close after 2 successes
});

const result = await breaker.execute(() => client.getNode('node-id'));
console.log('Circuit state:', breaker.getState());
```

## Connection Management

### Wait for Connection
```typescript
const client = new MemoryGraphClient({ address: 'localhost:50051' });

// Wait up to 5 seconds
await client.waitForReady(5000);

// Now safe to use
const session = await client.createSession();
```

### Health Monitoring
```typescript
// Start automatic health checks every 30 seconds
client.startHealthChecks(30000);

// Manual health check
const health = await client.health();
console.log('Status:', health.status);
console.log('Uptime:', health.uptimeSeconds);

// Stop health checks
client.stopHealthChecks();
```

### Graceful Shutdown
```typescript
try {
  await client.createSession();
} finally {
  // Always close when done
  await client.close(5000); // 5 second timeout
}
```

### Connection Status
```typescript
if (client.isConnected()) {
  console.log('Client is ready');
}

if (client.isClosing()) {
  console.log('Client is shutting down');
}
```

## Validation

### Automatic Validation
All methods automatically validate inputs:
```typescript
// ValidationError: sessionId cannot be empty
await client.getSession('');

// ValidationError: limit must be between 1 and 10000
await client.listSessions(0);
```

### Manual Validation
```typescript
import {
  validateSessionId,
  validateNodeId,
  validateTokenUsage,
  validateMetadata
} from '@llm-dev-ops/llm-memory-graph-client';

// Validate individual values
validateSessionId(sessionId);  // Throws ValidationError if invalid

// Validate complex objects
validateTokenUsage({
  promptTokens: 10,
  completionTokens: 20,
  totalTokens: 30
});
```

### Type Guards
```typescript
import { isPromptNode, isResponseNode } from '@llm-dev-ops/llm-memory-graph-client';

const node = await client.getNode('node-id');

if (isPromptNode(node)) {
  console.log('Prompt:', node.data.content);
}

if (isResponseNode(node)) {
  console.log('Tokens:', node.data.tokenUsage.totalTokens);
}
```

## Batch Operations

### Batch Get Nodes
```typescript
// Get multiple nodes efficiently
const nodeIds = ['id1', 'id2', 'id3'];
const nodes = await client.batchGetNodes(nodeIds);
```

### Batch Processing with Rate Limiting
```typescript
import { batchProcess } from '@llm-dev-ops/llm-memory-graph-client';

const results = await batchProcess(
  nodeIds,
  (id) => client.getNode(id),
  10,   // Process 10 at a time
  100   // Wait 100ms between batches
);
```

### Chunk Arrays
```typescript
import { chunk } from '@llm-dev-ops/llm-memory-graph-client';

const chunks = chunk(nodeIds, 10);
// [[id1...id10], [id11...id20], ...]

for (const batch of chunks) {
  await client.batchGetNodes(batch);
}
```

## Utilities

### Time Utilities
```typescript
import {
  sleep,
  formatDuration,
  timeDiff
} from '@llm-dev-ops/llm-memory-graph-client';

// Wait 1 second
await sleep(1000);

// Format duration
console.log(formatDuration(5432)); // "5.4s"
console.log(formatDuration(65000)); // "1m 5s"

// Calculate time difference
const start = new Date();
await someOperation();
const duration = timeDiff(start);
console.log(`Took ${formatDuration(duration)}`);
```

### Token Usage
```typescript
import { calculateTokenUsage } from '@llm-dev-ops/llm-memory-graph-client';

const total = calculateTokenUsage([
  { promptTokens: 10, completionTokens: 20, totalTokens: 30 },
  { promptTokens: 15, completionTokens: 25, totalTokens: 40 }
]);
console.log('Total tokens:', total.totalTokens); // 70
```

### Filter and Sort
```typescript
import {
  filterNodesByType,
  sortNodesByTime
} from '@llm-dev-ops/llm-memory-graph-client';

const results = await client.query({ sessionId: 'session-123' });

// Filter by type
const prompts = filterNodesByType(results.nodes, NodeType.PROMPT);

// Sort by time
const newest = sortNodesByTime(results.nodes, false); // Newest first
```

### Promise Utilities
```typescript
import { withTimeout } from '@llm-dev-ops/llm-memory-graph-client';

// Add timeout to any operation
const result = await withTimeout(
  client.getNode('node-id'),
  5000,
  'Operation timed out after 5s'
);
```

### Object Utilities
```typescript
import {
  deepClone,
  deepMerge,
  pick,
  omit
} from '@llm-dev-ops/llm-memory-graph-client';

// Deep clone
const copy = deepClone(original);

// Deep merge
const merged = deepMerge({ a: 1 }, { b: 2 });

// Pick properties
const subset = pick(obj, ['field1', 'field2']);

// Omit properties
const rest = omit(obj, ['password', 'secret']);
```

### Debounce and Throttle
```typescript
import { debounce, throttle } from '@llm-dev-ops/llm-memory-graph-client';

// Debounce - only execute after 300ms of no calls
const debouncedSearch = debounce((query: string) => {
  console.log('Searching:', query);
}, 300);

// Throttle - execute at most once per second
const throttledUpdate = throttle((value: number) => {
  console.log('Update:', value);
}, 1000);
```

## Complete Example

```typescript
import {
  MemoryGraphClient,
  ValidationError,
  NotFoundError,
  isMemoryGraphError,
  withTimeout,
  formatDuration
} from '@llm-dev-ops/llm-memory-graph-client';

async function main() {
  const client = new MemoryGraphClient({
    address: 'localhost:50051',
    retryPolicy: {
      maxRetries: 3,
      initialBackoff: 100,
      onRetry: (error, attempt, delay) => {
        console.log(`Retry ${attempt} after ${delay}ms`);
      }
    }
  });

  try {
    // Wait for connection
    await client.waitForReady(5000);

    // Start health monitoring
    client.startHealthChecks(30000);

    const start = new Date();

    // Create session with timeout
    const session = await withTimeout(
      client.createSession({
        metadata: { user: 'john' }
      }),
      5000
    );

    console.log('Session created:', session.id);
    console.log('Time taken:', formatDuration(timeDiff(start)));

  } catch (error) {
    if (error instanceof ValidationError) {
      console.error('Validation failed:', error.field);
    } else if (error instanceof NotFoundError) {
      console.error('Not found:', error.resourceId);
    } else if (isMemoryGraphError(error)) {
      console.error('Error:', error.code, error.message);
    } else {
      console.error('Unexpected error:', error);
    }
  } finally {
    // Always close gracefully
    await client.close();
  }
}

main().catch(console.error);
```

## Additional Resources

- [Full Documentation](./PRODUCTION-FEATURES.md)
- [API Reference](./README.md)
- [Examples](./examples/)
- [Type Definitions](./src/types.ts)
