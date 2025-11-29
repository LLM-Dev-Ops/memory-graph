/**
 * Production Features Example
 *
 * Demonstrates the production-ready features of the LLM Memory Graph Client:
 * - Error handling
 * - Retry logic with exponential backoff
 * - Input validation
 * - Connection management
 * - Health monitoring
 */

import {
  MemoryGraphClient,
  NodeType,
  ValidationError,
  ConnectionError,
  TimeoutError,
  NotFoundError,
  isMemoryGraphError,
  isRetryableError,
  withRetry,
  RetryManager,
  CircuitBreaker,
  CircuitState,
  batchProcess,
  formatDuration,
  isPromptNode,
  isResponseNode,
} from '../src';

async function main() {
  console.log('=== Production Features Demo ===\n');

  // ============================================================================
  // 1. Advanced Client Configuration
  // ============================================================================

  console.log('1. Creating client with advanced configuration...');
  const client = new MemoryGraphClient({
    address: 'localhost:50051',
    useTls: false,
    timeout: 30000,
    retryPolicy: {
      maxRetries: 5,
      initialBackoff: 100,
      maxBackoff: 10000,
      backoffMultiplier: 2,
      useJitter: true,
      onRetry: (error, attempt, delayMs) => {
        console.log(`  ⟳ Retry attempt ${attempt} after ${delayMs}ms`);
        console.log(`    Error: ${error instanceof Error ? error.message : String(error)}`);
      },
    },
  });

  try {
    // ============================================================================
    // 2. Connection Health Monitoring
    // ============================================================================

    console.log('\n2. Waiting for client to be ready...');
    await client.waitForReady(5000);
    console.log('  ✓ Client is ready!');

    console.log('\n3. Starting health checks (every 30 seconds)...');
    client.startHealthChecks(30000);
    console.log('  ✓ Health checks started');

    // ============================================================================
    // 3. Error Handling
    // ============================================================================

    console.log('\n4. Demonstrating error handling...');

    // Validation error
    try {
      await client.getSession(''); // Empty session ID
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log('  ✓ Caught ValidationError:', error.message);
        console.log('    Field:', error.field);
      }
    }

    // Not found error
    try {
      await client.getSession('non-existent-session-id');
    } catch (error) {
      if (error instanceof NotFoundError) {
        console.log('  ✓ Caught NotFoundError:', error.message);
        console.log('    Resource type:', error.resourceType);
        console.log('    Resource ID:', error.resourceId);
      }
    }

    // Generic error handling
    try {
      // Simulate an error
      throw new Error('Something went wrong');
    } catch (error) {
      if (isMemoryGraphError(error)) {
        console.log('  Memory Graph Error:', error.code, error.message);
      } else {
        console.log('  Generic error:', error);
      }
    }

    // ============================================================================
    // 4. Retry Logic with Custom Configuration
    // ============================================================================

    console.log('\n5. Demonstrating retry logic...');

    const retryManager = new RetryManager({
      maxRetries: 3,
      initialBackoff: 200,
      maxBackoff: 5000,
      onRetry: (error, attempt, delayMs) => {
        console.log(`  Retry ${attempt}: waiting ${delayMs}ms`);
      },
    });

    let attemptCount = 0;
    const result = await retryManager.execute(async (context) => {
      attemptCount++;
      console.log(`  Attempt ${attemptCount} (elapsed: ${context.elapsedMs}ms)`);

      // Simulate transient failures for first 2 attempts
      if (attemptCount < 3) {
        throw new ConnectionError('Simulated connection failure');
      }

      return 'Success!';
    });

    console.log('  ✓ Retry succeeded:', result);

    // ============================================================================
    // 5. Circuit Breaker Pattern
    // ============================================================================

    console.log('\n6. Demonstrating circuit breaker...');

    const breaker = new CircuitBreaker({
      failureThreshold: 3,
      resetTimeout: 5000,
      successThreshold: 2,
    });

    // Simulate some failures
    for (let i = 0; i < 3; i++) {
      try {
        await breaker.execute(async () => {
          throw new Error('Service unavailable');
        });
      } catch (error) {
        console.log(`  Attempt ${i + 1} failed`);
      }
    }

    const stats = breaker.getStats();
    console.log('  Circuit breaker state:', stats.state);
    console.log('  Failure count:', stats.failureCount);

    if (stats.state === CircuitState.OPEN) {
      console.log('  ✓ Circuit is now OPEN (preventing further calls)');
    }

    // ============================================================================
    // 6. Working with Sessions
    // ============================================================================

    console.log('\n7. Creating session with automatic retry...');
    const session = await client.createSession({
      metadata: {
        user: 'demo-user',
        environment: 'production',
        version: '1.0.0',
      },
    });
    console.log('  ✓ Session created:', session.id);
    console.log('    Created at:', session.createdAt.toISOString());

    // ============================================================================
    // 7. Adding Prompts and Responses
    // ============================================================================

    console.log('\n8. Adding prompt with validation...');
    const prompt = await client.addPrompt({
      sessionId: session.id,
      content: 'What are the production-ready features of this SDK?',
      metadata: {
        model: 'gpt-4',
        temperature: 0.7,
        toolsAvailable: ['search', 'code_analysis'],
        custom: {
          source: 'demo',
          priority: 'high',
        },
      },
    });
    console.log('  ✓ Prompt added:', prompt.id);

    console.log('\n9. Adding response with token usage...');
    const response = await client.addResponse({
      promptId: prompt.id,
      content: 'The SDK includes: error handling, retry logic, validation, and more!',
      tokenUsage: {
        promptTokens: 25,
        completionTokens: 30,
        totalTokens: 55,
      },
      metadata: {
        model: 'gpt-4',
        finishReason: 'stop',
        latencyMs: 1234,
        custom: {
          cached: 'false',
        },
      },
    });
    console.log('  ✓ Response added:', response.id);
    console.log('    Tokens used:', response.tokenUsage.totalTokens);

    // ============================================================================
    // 8. Batch Processing
    // ============================================================================

    console.log('\n10. Demonstrating batch processing...');

    // Create multiple prompts
    const promptIds = [];
    for (let i = 0; i < 5; i++) {
      const p = await client.addPrompt({
        sessionId: session.id,
        content: `Test prompt ${i + 1}`,
        metadata: {
          model: 'gpt-4',
          temperature: 0.7,
          toolsAvailable: [],
          custom: {},
        },
      });
      promptIds.push(p.id);
    }

    // Batch process with rate limiting
    console.log('  Processing 5 prompts in batches of 2...');
    const startTime = new Date();
    const results = await batchProcess(
      promptIds,
      async (id) => {
        console.log(`    Fetching node ${id.substring(0, 8)}...`);
        return await client.getNode(id);
      },
      2, // Process 2 at a time
      100 // Wait 100ms between batches
    );
    const duration = new Date().getTime() - startTime.getTime();
    console.log(`  ✓ Processed ${results.length} nodes in ${formatDuration(duration)}`);

    // ============================================================================
    // 9. Querying with Validation
    // ============================================================================

    console.log('\n11. Querying nodes with type filtering...');
    const queryResults = await client.query({
      sessionId: session.id,
      nodeType: NodeType.PROMPT,
      limit: 10,
    });
    console.log(`  ✓ Found ${queryResults.totalCount} prompt nodes`);

    // Type guards
    let promptCount = 0;
    let responseCount = 0;
    for (const node of queryResults.nodes) {
      if (isPromptNode(node)) promptCount++;
      if (isResponseNode(node)) responseCount++;
    }
    console.log(`    Prompts: ${promptCount}, Responses: ${responseCount}`);

    // ============================================================================
    // 10. Cleanup
    // ============================================================================

    console.log('\n12. Cleaning up...');
    await client.deleteSession(session.id);
    console.log('  ✓ Session deleted');

  } catch (error) {
    console.error('\n❌ Error occurred:');

    if (isMemoryGraphError(error)) {
      console.error('  Type:', error.name);
      console.error('  Code:', error.code);
      console.error('  Message:', error.message);
      console.error('  Status:', error.statusCode);
      console.error('  Details:', error.details);
      console.error('  Retryable:', isRetryableError(error));
    } else {
      console.error('  Unknown error:', error);
    }
  } finally {
    // ============================================================================
    // 11. Graceful Shutdown
    // ============================================================================

    console.log('\n13. Closing client gracefully...');
    client.stopHealthChecks();
    await client.close(5000);
    console.log('  ✓ Client closed successfully');
  }

  console.log('\n=== Demo Complete ===');
}

// Run the demo
if (require.main === module) {
  main().catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}

export { main };
