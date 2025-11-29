/**
 * Mock data and fixtures for testing
 */

import {
  Session,
  Node,
  Edge,
  NodeType,
  EdgeType,
  PromptNode,
  ResponseNode,
  TokenUsage,
  PromptMetadata,
  ResponseMetadata,
} from '../../src/types';

/**
 * Mock session data
 */
export const mockSession: Session = {
  id: 'test-session-123',
  createdAt: new Date('2024-01-01T00:00:00.000Z'),
  updatedAt: new Date('2024-01-01T00:00:00.000Z'),
  metadata: {
    user: 'test-user',
    context: 'test',
  },
  isActive: true,
};

/**
 * Mock prompt metadata
 */
export const mockPromptMetadata: PromptMetadata = {
  model: 'gpt-4',
  temperature: 0.7,
  maxTokens: 1000,
  toolsAvailable: ['search', 'calculator'],
  custom: {
    environment: 'test',
  },
};

/**
 * Mock response metadata
 */
export const mockResponseMetadata: ResponseMetadata = {
  model: 'gpt-4',
  finishReason: 'stop',
  latencyMs: 1234,
  custom: {
    environment: 'test',
  },
};

/**
 * Mock token usage
 */
export const mockTokenUsage: TokenUsage = {
  promptTokens: 10,
  completionTokens: 20,
  totalTokens: 30,
};

/**
 * Mock prompt node data
 */
export const mockPromptNodeData: PromptNode = {
  id: 'prompt-node-123',
  sessionId: 'test-session-123',
  content: 'What is the capital of France?',
  timestamp: new Date('2024-01-01T00:00:00.000Z'),
  metadata: mockPromptMetadata,
};

/**
 * Mock response node data
 */
export const mockResponseNodeData: ResponseNode = {
  id: 'response-node-123',
  promptId: 'prompt-node-123',
  content: 'The capital of France is Paris.',
  timestamp: new Date('2024-01-01T00:01:00.000Z'),
  tokenUsage: mockTokenUsage,
  metadata: mockResponseMetadata,
};

/**
 * Mock prompt node
 */
export const mockPromptNode: Node = {
  id: 'prompt-node-123',
  type: NodeType.PROMPT,
  createdAt: new Date('2024-01-01T00:00:00.000Z'),
  data: mockPromptNodeData,
};

/**
 * Mock response node
 */
export const mockResponseNode: Node = {
  id: 'response-node-123',
  type: NodeType.RESPONSE,
  createdAt: new Date('2024-01-01T00:01:00.000Z'),
  data: mockResponseNodeData,
};

/**
 * Mock edge
 */
export const mockEdge: Edge = {
  id: 'edge-123',
  fromNodeId: 'prompt-node-123',
  toNodeId: 'response-node-123',
  type: EdgeType.RESPONDS_TO,
  createdAt: new Date('2024-01-01T00:01:00.000Z'),
  properties: {
    custom: 'value',
  },
};

/**
 * Generate mock session
 */
export function generateMockSession(overrides?: Partial<Session>): Session {
  return {
    ...mockSession,
    ...overrides,
    id: overrides?.id || `session-${Math.random().toString(36).substr(2, 9)}`,
  };
}

/**
 * Generate mock node
 */
export function generateMockNode(type: NodeType = NodeType.PROMPT, overrides?: Partial<Node>): Node {
  const baseNode: Node = {
    id: `node-${Math.random().toString(36).substr(2, 9)}`,
    type,
    createdAt: new Date(),
    data: type === NodeType.PROMPT ? mockPromptNodeData : mockResponseNodeData,
  };

  return {
    ...baseNode,
    ...overrides,
  };
}

/**
 * Generate mock edge
 */
export function generateMockEdge(overrides?: Partial<Edge>): Edge {
  return {
    ...mockEdge,
    ...overrides,
    id: overrides?.id || `edge-${Math.random().toString(36).substr(2, 9)}`,
  };
}

/**
 * Generate mock token usage
 */
export function generateMockTokenUsage(overrides?: Partial<TokenUsage>): TokenUsage {
  const promptTokens = overrides?.promptTokens ?? 10;
  const completionTokens = overrides?.completionTokens ?? 20;
  return {
    promptTokens,
    completionTokens,
    totalTokens: overrides?.totalTokens ?? promptTokens + completionTokens,
  };
}

/**
 * Mock gRPC error
 */
export function createMockGrpcError(code: number, message: string, metadata?: Record<string, string>) {
  const error: any = new Error(message);
  error.code = code;
  if (metadata) {
    error.metadata = {
      getMap: () => new Map(Object.entries(metadata)),
    };
  }
  return error;
}

/**
 * Valid UUID for testing
 */
export const VALID_UUID = '123e4567-e89b-12d3-a456-426614174000';

/**
 * Invalid UUID for testing
 */
export const INVALID_UUID = 'not-a-uuid';
