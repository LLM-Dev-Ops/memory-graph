/**
 * Conversation Memory Agent - Unit Tests
 */

import { describe, it, expect, beforeEach, jest } from '@jest/globals';
import { ConversationMemoryAgent } from '../agent.js';
import type { ConversationCaptureInput, DecisionEvent } from '../types.js';

// Mock crypto for Node.js environment
const mockRandomUUID = jest.fn(() => '12345678-1234-1234-1234-123456789012');
const mockSubtleDigest = jest.fn(async () => new Uint8Array(32).fill(0));

global.crypto = {
  randomUUID: mockRandomUUID,
  subtle: {
    digest: mockSubtleDigest,
  },
} as unknown as Crypto;

describe('ConversationMemoryAgent', () => {
  let agent: ConversationMemoryAgent;
  let mockRuVectorClient: {
    persistDecisionEvent: jest.Mock;
    retrieveDecisionEvent: jest.Mock;
    queryDecisionEvents: jest.Mock;
    healthCheck: jest.Mock;
  };

  beforeEach(() => {
    mockRuVectorClient = {
      persistDecisionEvent: jest.fn().mockResolvedValue({
        success: true,
        event_id: 'event-123',
        latency_ms: 50,
      }),
      retrieveDecisionEvent: jest.fn().mockResolvedValue(null),
      queryDecisionEvents: jest.fn().mockResolvedValue([]),
      healthCheck: jest.fn().mockResolvedValue(true),
    };

    agent = new ConversationMemoryAgent(mockRuVectorClient as unknown as any);
  });

  describe('execute', () => {
    it('should successfully capture a valid conversation', async () => {
      const input: ConversationCaptureInput = {
        session_id: '12345678-1234-1234-1234-123456789012',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [
          {
            turn_id: '12345678-1234-1234-1234-123456789014',
            role: 'user',
            content: 'Hello, how are you?',
            timestamp: '2024-01-15T10:00:00.000Z',
          },
          {
            turn_id: '12345678-1234-1234-1234-123456789015',
            role: 'assistant',
            content: 'I am doing well, thank you!',
            timestamp: '2024-01-15T10:00:01.000Z',
            token_usage: {
              prompt_tokens: 10,
              completion_tokens: 8,
              total_tokens: 18,
            },
          },
        ],
      };

      const result = await agent.execute(input);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.output.turn_count).toBe(2);
        expect(result.output.session_id).toBe(input.session_id);
        expect(result.output.conversation_id).toBe(input.conversation_id);
        expect(result.output.nodes_created.length).toBeGreaterThan(0);
        expect(result.output.edges_created.length).toBeGreaterThan(0);
        expect(result.decisionEvent.agent_id).toBe('conversation-memory-agent');
        expect(result.decisionEvent.decision_type).toBe('conversation_capture');
        expect(result.decisionEvent.confidence).toBe(1.0);
      }
    });

    it('should create session membership edges for all turns', async () => {
      const input: ConversationCaptureInput = {
        session_id: '12345678-1234-1234-1234-123456789012',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [
          {
            turn_id: '12345678-1234-1234-1234-123456789014',
            role: 'user',
            content: 'First message',
            timestamp: '2024-01-15T10:00:00.000Z',
          },
          {
            turn_id: '12345678-1234-1234-1234-123456789015',
            role: 'assistant',
            content: 'Response',
            timestamp: '2024-01-15T10:00:01.000Z',
          },
          {
            turn_id: '12345678-1234-1234-1234-123456789016',
            role: 'user',
            content: 'Follow up',
            timestamp: '2024-01-15T10:00:02.000Z',
          },
        ],
      };

      const result = await agent.execute(input);

      expect(result.success).toBe(true);
      if (result.success) {
        const sessionEdges = result.output.edges_created.filter(
          (e) => e.edge_type === 'belongs_to'
        );
        // Each turn should have a belongs_to edge to session
        expect(sessionEdges.length).toBe(3);
      }
    });

    it('should create lineage edges when enabled', async () => {
      const input: ConversationCaptureInput = {
        session_id: '12345678-1234-1234-1234-123456789012',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [
          {
            turn_id: '12345678-1234-1234-1234-123456789014',
            role: 'user',
            content: 'Question',
            timestamp: '2024-01-15T10:00:00.000Z',
          },
          {
            turn_id: '12345678-1234-1234-1234-123456789015',
            role: 'assistant',
            content: 'Answer',
            timestamp: '2024-01-15T10:00:01.000Z',
          },
        ],
        capture_options: {
          create_lineage: true,
        },
      };

      const result = await agent.execute(input);

      expect(result.success).toBe(true);
      if (result.success) {
        const lineageEdges = result.output.edges_created.filter(
          (e) => e.edge_type === 'responds_to' || e.edge_type === 'follows'
        );
        expect(lineageEdges.length).toBeGreaterThan(0);
      }
    });

    it('should return validation error for invalid input', async () => {
      const invalidInput = {
        session_id: 'not-a-uuid',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [],
      };

      const result = await agent.execute(invalidInput);

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.error_code).toBe('VALIDATION_ERROR');
      }
    });

    it('should return validation error when turns array is empty', async () => {
      const input = {
        session_id: '12345678-1234-1234-1234-123456789012',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [],
      };

      const result = await agent.execute(input);

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.error_code).toBe('VALIDATION_ERROR');
      }
    });

    it('should handle ruvector persistence failure', async () => {
      mockRuVectorClient.persistDecisionEvent.mockResolvedValue({
        success: false,
        error: 'Connection refused',
      });

      const input: ConversationCaptureInput = {
        session_id: '12345678-1234-1234-1234-123456789012',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [
          {
            turn_id: '12345678-1234-1234-1234-123456789014',
            role: 'user',
            content: 'Test',
            timestamp: '2024-01-15T10:00:00.000Z',
          },
        ],
      };

      const result = await agent.execute(input);

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.error_code).toBe('RUVECTOR_WRITE_ERROR');
      }
    });

    it('should process tool invocations', async () => {
      const input: ConversationCaptureInput = {
        session_id: '12345678-1234-1234-1234-123456789012',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [
          {
            turn_id: '12345678-1234-1234-1234-123456789014',
            role: 'assistant',
            content: 'Using calculator tool',
            timestamp: '2024-01-15T10:00:00.000Z',
            tool_invocations: [
              {
                tool_name: 'calculator',
                invocation_id: '12345678-1234-1234-1234-123456789020',
                success: true,
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      expect(result.success).toBe(true);
      if (result.success) {
        const toolNodes = result.output.nodes_created.filter(
          (n) => n.node_type === 'tool_invocation'
        );
        const invokeEdges = result.output.edges_created.filter(
          (e) => e.edge_type === 'invokes'
        );
        expect(toolNodes.length).toBe(1);
        expect(invokeEdges.length).toBe(1);
      }
    });

    it('should calculate total tokens correctly', async () => {
      const input: ConversationCaptureInput = {
        session_id: '12345678-1234-1234-1234-123456789012',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [
          {
            turn_id: '12345678-1234-1234-1234-123456789014',
            role: 'assistant',
            content: 'First response',
            timestamp: '2024-01-15T10:00:00.000Z',
            token_usage: {
              prompt_tokens: 10,
              completion_tokens: 15,
              total_tokens: 25,
            },
          },
          {
            turn_id: '12345678-1234-1234-1234-123456789015',
            role: 'assistant',
            content: 'Second response',
            timestamp: '2024-01-15T10:00:01.000Z',
            token_usage: {
              prompt_tokens: 20,
              completion_tokens: 30,
              total_tokens: 50,
            },
          },
        ],
      };

      const result = await agent.execute(input);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.output.total_tokens).toBe(75);
      }
    });

    it('should map roles to correct node types', async () => {
      const input: ConversationCaptureInput = {
        session_id: '12345678-1234-1234-1234-123456789012',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [
          {
            turn_id: '12345678-1234-1234-1234-123456789014',
            role: 'user',
            content: 'User message',
            timestamp: '2024-01-15T10:00:00.000Z',
          },
          {
            turn_id: '12345678-1234-1234-1234-123456789015',
            role: 'assistant',
            content: 'Assistant message',
            timestamp: '2024-01-15T10:00:01.000Z',
          },
          {
            turn_id: '12345678-1234-1234-1234-123456789016',
            role: 'system',
            content: 'System message',
            timestamp: '2024-01-15T10:00:02.000Z',
          },
        ],
      };

      const result = await agent.execute(input);

      expect(result.success).toBe(true);
      if (result.success) {
        const turnNodes = result.output.nodes_created.filter(
          (n) => n.turn_index !== undefined
        );

        const userNode = turnNodes.find((n) => n.turn_index === 0);
        const assistantNode = turnNodes.find((n) => n.turn_index === 1);
        const systemNode = turnNodes.find((n) => n.turn_index === 2);

        expect(userNode?.node_type).toBe('prompt');
        expect(assistantNode?.node_type).toBe('response');
        expect(systemNode?.node_type).toBe('prompt');
      }
    });
  });

  describe('DecisionEvent', () => {
    it('should emit exactly one DecisionEvent per invocation', async () => {
      const input: ConversationCaptureInput = {
        session_id: '12345678-1234-1234-1234-123456789012',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [
          {
            turn_id: '12345678-1234-1234-1234-123456789014',
            role: 'user',
            content: 'Test',
            timestamp: '2024-01-15T10:00:00.000Z',
          },
        ],
      };

      await agent.execute(input);

      expect(mockRuVectorClient.persistDecisionEvent).toHaveBeenCalledTimes(1);
    });

    it('should include correct agent metadata in DecisionEvent', async () => {
      const input: ConversationCaptureInput = {
        session_id: '12345678-1234-1234-1234-123456789012',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [
          {
            turn_id: '12345678-1234-1234-1234-123456789014',
            role: 'user',
            content: 'Test',
            timestamp: '2024-01-15T10:00:00.000Z',
          },
        ],
      };

      const result = await agent.execute(input);

      expect(result.success).toBe(true);
      if (result.success) {
        const event = result.decisionEvent;
        expect(event.agent_id).toBe('conversation-memory-agent');
        expect(event.agent_version).toMatch(/^\d+\.\d+\.\d+$/);
        expect(event.decision_type).toBe('conversation_capture');
        expect(event.inputs_hash).toHaveLength(64); // SHA-256 hex
        expect(event.confidence).toBe(1.0);
        expect(event.execution_ref).toBeDefined();
        expect(event.timestamp).toBeDefined();
      }
    });

    it('should track constraints applied', async () => {
      const input: ConversationCaptureInput = {
        session_id: '12345678-1234-1234-1234-123456789012',
        conversation_id: '12345678-1234-1234-1234-123456789013',
        turns: [
          {
            turn_id: '12345678-1234-1234-1234-123456789014',
            role: 'user',
            content: 'Test',
            timestamp: '2024-01-15T10:00:00.000Z',
          },
        ],
        capture_options: {
          create_lineage: true,
          extract_entities: true,
          compute_embeddings: true,
        },
      };

      const result = await agent.execute(input);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.decisionEvent.constraints_applied).toContain('lineage_creation_enabled');
        expect(result.decisionEvent.constraints_applied).toContain('entity_extraction_enabled');
        expect(result.decisionEvent.constraints_applied).toContain('embedding_computation_deferred');
      }
    });
  });
});
