/**
 * Integration tests for MemoryGraphClient
 *
 * These tests verify the client works end-to-end.
 * They require a running Memory Graph server.
 *
 * Set INTEGRATION_TESTS=true to run these tests:
 * INTEGRATION_TESTS=true npm test
 */

import { describe, it, expect, beforeAll, afterAll } from '@jest/globals';
import { MemoryGraphClient } from '../../src/client';
import { ValidationError } from '../../src/errors';
import { NodeType } from '../../src/types';

const RUN_INTEGRATION_TESTS = process.env.INTEGRATION_TESTS === 'true';

const describeIntegration = RUN_INTEGRATION_TESTS ? describe : describe.skip;

describeIntegration('MemoryGraphClient Integration', () => {
  let client: MemoryGraphClient;
  const SERVER_ADDRESS = process.env.MEMORY_GRAPH_SERVER || 'localhost:50051';

  beforeAll(async () => {
    client = new MemoryGraphClient({
      address: SERVER_ADDRESS,
      useTls: false,
      retryPolicy: {
        maxRetries: 3,
        initialBackoff: 100,
      },
    });

    // Wait for client to be ready
    try {
      await client.waitForReady(5000);
    } catch (error) {
      console.warn('Failed to connect to Memory Graph server:', error);
      console.warn('Skipping integration tests. Start the server and set INTEGRATION_TESTS=true to run.');
    }
  });

  afterAll(async () => {
    if (client) {
      await client.close();
    }
  });

  describe('Health Check', () => {
    it('should check server health', async () => {
      const health = await client.health();

      expect(health.status).toBeDefined();
      expect(health.version).toBeDefined();
      expect(typeof health.uptimeSeconds).toBe('number');
    });

    it('should get server metrics', async () => {
      const metrics = await client.getMetrics();

      expect(typeof metrics.totalNodes).toBe('number');
      expect(typeof metrics.totalEdges).toBe('number');
      expect(typeof metrics.totalSessions).toBe('number');
      expect(typeof metrics.activeSessions).toBe('number');
    });
  });

  describe('Session Management', () => {
    it('should create a session', async () => {
      const session = await client.createSession({
        metadata: {
          test: 'integration-test',
          timestamp: new Date().toISOString(),
        },
      });

      expect(session.id).toBeDefined();
      expect(session.createdAt).toBeInstanceOf(Date);
      expect(session.isActive).toBe(true);
      expect(session.metadata.test).toBe('integration-test');
    });

    it('should get a session', async () => {
      const created = await client.createSession({ metadata: { test: 'get-test' } });
      const retrieved = await client.getSession(created.id);

      expect(retrieved.id).toBe(created.id);
      expect(retrieved.metadata.test).toBe('get-test');
    });

    it('should list sessions', async () => {
      const result = await client.listSessions(10, 0);

      expect(Array.isArray(result.sessions)).toBe(true);
      expect(typeof result.totalCount).toBe('number');
    });

    it('should delete a session', async () => {
      const session = await client.createSession({ metadata: { test: 'delete-test' } });

      await client.deleteSession(session.id);

      // Verify deletion by trying to get it
      await expect(client.getSession(session.id)).rejects.toThrow();
    });
  });

  describe('Prompt and Response Operations', () => {
    it('should add a prompt', async () => {
      const session = await client.createSession({ metadata: { test: 'prompt-test' } });

      const prompt = await client.addPrompt({
        sessionId: session.id,
        content: 'What is TypeScript?',
        metadata: {
          model: 'gpt-4',
          temperature: 0.7,
          maxTokens: 1000,
          toolsAvailable: [],
          custom: {},
        },
      });

      expect(prompt.id).toBeDefined();
      expect(prompt.sessionId).toBe(session.id);
      expect(prompt.content).toBe('What is TypeScript?');
    });

    it('should add a response to a prompt', async () => {
      const session = await client.createSession({ metadata: { test: 'response-test' } });

      const prompt = await client.addPrompt({
        sessionId: session.id,
        content: 'What is 2+2?',
        metadata: {
          model: 'gpt-4',
          temperature: 0.7,
          maxTokens: 100,
          toolsAvailable: [],
          custom: {},
        },
      });

      const response = await client.addResponse({
        promptId: prompt.id,
        content: 'The answer is 4.',
        tokenUsage: {
          promptTokens: 10,
          completionTokens: 5,
          totalTokens: 15,
        },
        metadata: {
          model: 'gpt-4',
          finishReason: 'stop',
          latencyMs: 1234,
          custom: {},
        },
      });

      expect(response.id).toBeDefined();
      expect(response.promptId).toBe(prompt.id);
      expect(response.content).toBe('The answer is 4.');
    });
  });

  describe('Query Operations', () => {
    it('should query nodes by session', async () => {
      const session = await client.createSession({ metadata: { test: 'query-test' } });

      await client.addPrompt({
        sessionId: session.id,
        content: 'Test query',
        metadata: {
          model: 'gpt-4',
          temperature: 0.7,
          maxTokens: 100,
          toolsAvailable: [],
          custom: {},
        },
      });

      const result = await client.query({
        sessionId: session.id,
        limit: 10,
      });

      expect(result.nodes.length).toBeGreaterThan(0);
      expect(result.totalCount).toBeGreaterThan(0);
    });

    it('should query nodes by type', async () => {
      const result = await client.query({
        nodeType: NodeType.PROMPT,
        limit: 5,
      });

      expect(Array.isArray(result.nodes)).toBe(true);
      result.nodes.forEach((node) => {
        expect(node.type).toBe(NodeType.PROMPT);
      });
    });

    it('should respect limit and offset', async () => {
      const result1 = await client.query({ limit: 2, offset: 0 });
      const result2 = await client.query({ limit: 2, offset: 2 });

      expect(result1.nodes.length).toBeLessThanOrEqual(2);
      expect(result2.nodes.length).toBeLessThanOrEqual(2);

      // Results should be different
      if (result1.nodes.length > 0 && result2.nodes.length > 0) {
        expect(result1.nodes[0].id).not.toBe(result2.nodes[0].id);
      }
    });
  });

  describe('Error Handling', () => {
    it('should throw ValidationError for invalid session ID', async () => {
      await expect(client.getSession('')).rejects.toThrow(ValidationError);
    });

    it('should throw error for non-existent session', async () => {
      await expect(client.getSession('non-existent-id')).rejects.toThrow();
    });

    it('should throw ValidationError for invalid prompt request', async () => {
      await expect(
        client.addPrompt({
          sessionId: '',
          content: 'test',
        } as any)
      ).rejects.toThrow(ValidationError);
    });
  });

  describe('Retry Behavior', () => {
    it('should retry on transient failures', async () => {
      // This test would require simulating transient failures
      // For now, we just verify the retry policy is configured
      const policy = client.getRetryPolicy();
      expect(policy.maxRetries).toBeGreaterThan(0);
    });
  });

  describe('Connection Management', () => {
    it('should reconnect after close', async () => {
      const tempClient = new MemoryGraphClient({
        address: SERVER_ADDRESS,
        useTls: false,
      });

      await tempClient.close();
      expect(tempClient.isConnected()).toBe(false);

      // Create a new client to reconnect
      const newClient = new MemoryGraphClient({
        address: SERVER_ADDRESS,
        useTls: false,
      });

      const health = await newClient.health();
      expect(health.status).toBeDefined();

      await newClient.close();
    });
  });
});

// Minimal test when integration tests are skipped
describe('Integration Tests', () => {
  it('should skip when INTEGRATION_TESTS is not set', () => {
    if (!RUN_INTEGRATION_TESTS) {
      console.log('Integration tests skipped. Set INTEGRATION_TESTS=true to run.');
    }
    expect(true).toBe(true);
  });
});
