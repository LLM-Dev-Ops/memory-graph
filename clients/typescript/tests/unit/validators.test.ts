/**
 * Unit tests for validators module
 */

import { describe, it, expect } from '@jest/globals';
import { ValidationError } from '../../src/errors';
import { NodeType, EdgeType, EdgeDirection } from '../../src/types';
import {
  validateNonEmptyString,
  validateUuid,
  validatePositiveNumber,
  validateNonNegativeInteger,
  validateRange,
  validateDate,
  validateObject,
  validateEnum,
  validateSessionId,
  validateNodeId,
  validateEdgeId,
  validateMetadata,
  validateTokenUsage,
  validatePromptMetadata,
  validateResponseMetadata,
  validateAddPromptRequest,
  validateAddResponseRequest,
  validateQueryOptions,
  validateNode,
  validateEdge,
  validateEdgeDirection,
  validateEdgeType,
  validateNodeArray,
  validateNodeIdArray,
  validateLimit,
  validateOffset,
  sanitizeInput,
  validateJsonString,
} from '../../src/validators';
import { VALID_UUID, INVALID_UUID, mockPromptMetadata, mockResponseMetadata, mockTokenUsage } from '../fixtures/mock-data';

describe('validateNonEmptyString', () => {
  it('should accept valid non-empty string', () => {
    expect(() => validateNonEmptyString('hello', 'field')).not.toThrow();
  });

  it('should reject empty string', () => {
    expect(() => validateNonEmptyString('', 'field')).toThrow(ValidationError);
    expect(() => validateNonEmptyString('', 'field')).toThrow('field cannot be empty');
  });

  it('should reject whitespace-only string', () => {
    expect(() => validateNonEmptyString('   ', 'field')).toThrow(ValidationError);
  });

  it('should reject non-string values', () => {
    expect(() => validateNonEmptyString(123, 'field')).toThrow(ValidationError);
    expect(() => validateNonEmptyString(null, 'field')).toThrow('field must be a string');
    expect(() => validateNonEmptyString(undefined, 'field')).toThrow(ValidationError);
  });
});

describe('validateUuid', () => {
  it('should accept valid UUID', () => {
    expect(() => validateUuid(VALID_UUID, 'field')).not.toThrow();
  });

  it('should reject invalid UUID format', () => {
    expect(() => validateUuid(INVALID_UUID, 'field')).toThrow(ValidationError);
    expect(() => validateUuid(INVALID_UUID, 'field')).toThrow('field must be a valid UUID');
  });

  it('should reject non-string values', () => {
    expect(() => validateUuid(123, 'field')).toThrow(ValidationError);
  });
});

describe('validatePositiveNumber', () => {
  it('should accept positive numbers', () => {
    expect(() => validatePositiveNumber(1, 'field')).not.toThrow();
    expect(() => validatePositiveNumber(0.5, 'field')).not.toThrow();
    expect(() => validatePositiveNumber(0, 'field')).not.toThrow(); // Zero is non-negative
  });

  it('should reject negative numbers', () => {
    expect(() => validatePositiveNumber(-1, 'field')).toThrow(ValidationError);
    expect(() => validatePositiveNumber(-1, 'field')).toThrow('field must be positive');
  });

  it('should reject non-finite numbers', () => {
    expect(() => validatePositiveNumber(Infinity, 'field')).toThrow(ValidationError);
    expect(() => validatePositiveNumber(NaN, 'field')).toThrow('field must be a finite number');
  });

  it('should reject non-number values', () => {
    expect(() => validatePositiveNumber('123', 'field')).toThrow(ValidationError);
    expect(() => validatePositiveNumber('123', 'field')).toThrow('field must be a number');
  });
});

describe('validateNonNegativeInteger', () => {
  it('should accept non-negative integers', () => {
    expect(() => validateNonNegativeInteger(0, 'field')).not.toThrow();
    expect(() => validateNonNegativeInteger(1, 'field')).not.toThrow();
    expect(() => validateNonNegativeInteger(100, 'field')).not.toThrow();
  });

  it('should reject negative integers', () => {
    expect(() => validateNonNegativeInteger(-1, 'field')).toThrow(ValidationError);
  });

  it('should reject non-integers', () => {
    expect(() => validateNonNegativeInteger(1.5, 'field')).toThrow(ValidationError);
    expect(() => validateNonNegativeInteger(1.5, 'field')).toThrow('field must be an integer');
  });
});

describe('validateRange', () => {
  it('should accept values within range', () => {
    expect(() => validateRange(5, 'field', 0, 10)).not.toThrow();
    expect(() => validateRange(0, 'field', 0, 10)).not.toThrow();
    expect(() => validateRange(10, 'field', 0, 10)).not.toThrow();
  });

  it('should reject values outside range', () => {
    expect(() => validateRange(-1, 'field', 0, 10)).toThrow(ValidationError);
    expect(() => validateRange(11, 'field', 0, 10)).toThrow(ValidationError);
    expect(() => validateRange(11, 'field', 0, 10)).toThrow('field must be between 0 and 10');
  });
});

describe('validateDate', () => {
  it('should accept valid Date objects', () => {
    expect(() => validateDate(new Date(), 'field')).not.toThrow();
    expect(() => validateDate(new Date('2024-01-01'), 'field')).not.toThrow();
  });

  it('should reject invalid Date objects', () => {
    expect(() => validateDate(new Date('invalid'), 'field')).toThrow(ValidationError);
    expect(() => validateDate(new Date('invalid'), 'field')).toThrow('field must be a valid Date');
  });

  it('should reject non-Date values', () => {
    expect(() => validateDate('2024-01-01', 'field')).toThrow(ValidationError);
    expect(() => validateDate(123456789, 'field')).toThrow('field must be a Date object');
  });
});

describe('validateObject', () => {
  it('should accept valid objects', () => {
    expect(() => validateObject({}, 'field')).not.toThrow();
    expect(() => validateObject({ key: 'value' }, 'field')).not.toThrow();
  });

  it('should reject null', () => {
    expect(() => validateObject(null, 'field')).toThrow(ValidationError);
  });

  it('should reject arrays', () => {
    expect(() => validateObject([], 'field')).toThrow(ValidationError);
  });

  it('should reject primitives', () => {
    expect(() => validateObject('string', 'field')).toThrow(ValidationError);
    expect(() => validateObject(123, 'field')).toThrow('field must be an object');
  });
});

describe('validateEnum', () => {
  it('should accept valid enum values', () => {
    expect(() => validateEnum(NodeType.PROMPT, 'field', NodeType)).not.toThrow();
    expect(() => validateEnum(NodeType.RESPONSE, 'field', NodeType)).not.toThrow();
  });

  it('should reject invalid enum values', () => {
    expect(() => validateEnum('INVALID', 'field', NodeType)).toThrow(ValidationError);
    expect(() => validateEnum('INVALID', 'field', NodeType)).toThrow('field must be one of');
  });
});

describe('ID Validators', () => {
  it('should validate session ID', () => {
    expect(() => validateSessionId('session-123')).not.toThrow();
    expect(() => validateSessionId('')).toThrow(ValidationError);
  });

  it('should validate node ID', () => {
    expect(() => validateNodeId('node-123')).not.toThrow();
    expect(() => validateNodeId('')).toThrow(ValidationError);
  });

  it('should validate edge ID', () => {
    expect(() => validateEdgeId('edge-123')).not.toThrow();
    expect(() => validateEdgeId('')).toThrow(ValidationError);
  });
});

describe('validateMetadata', () => {
  it('should accept valid metadata', () => {
    const metadata = { key1: 'value1', key2: 'value2' };
    expect(() => validateMetadata(metadata, 'metadata')).not.toThrow();
  });

  it('should accept empty metadata', () => {
    expect(() => validateMetadata({}, 'metadata')).not.toThrow();
  });

  it('should reject non-string values', () => {
    const metadata = { key1: 'value1', key2: 123 };
    expect(() => validateMetadata(metadata, 'metadata')).toThrow(ValidationError);
    expect(() => validateMetadata(metadata, 'metadata')).toThrow('metadata.key2 must be a string');
  });
});

describe('validateTokenUsage', () => {
  it('should accept valid token usage', () => {
    expect(() => validateTokenUsage(mockTokenUsage)).not.toThrow();
  });

  it('should reject invalid total', () => {
    const invalid = { promptTokens: 10, completionTokens: 20, totalTokens: 25 };
    expect(() => validateTokenUsage(invalid)).toThrow(ValidationError);
    expect(() => validateTokenUsage(invalid)).toThrow('totalTokens must equal');
  });

  it('should reject negative values', () => {
    const invalid = { promptTokens: -1, completionTokens: 20, totalTokens: 19 };
    expect(() => validateTokenUsage(invalid)).toThrow(ValidationError);
  });

  it('should reject non-integers', () => {
    const invalid = { promptTokens: 10.5, completionTokens: 20, totalTokens: 30.5 };
    expect(() => validateTokenUsage(invalid)).toThrow(ValidationError);
  });
});

describe('validatePromptMetadata', () => {
  it('should accept valid prompt metadata', () => {
    expect(() => validatePromptMetadata(mockPromptMetadata)).not.toThrow();
  });

  it('should reject invalid model', () => {
    const invalid = { ...mockPromptMetadata, model: '' };
    expect(() => validatePromptMetadata(invalid)).toThrow(ValidationError);
  });

  it('should reject invalid temperature', () => {
    const invalid = { ...mockPromptMetadata, temperature: 3 };
    expect(() => validatePromptMetadata(invalid)).toThrow(ValidationError);
    expect(() => validatePromptMetadata(invalid)).toThrow('temperature must be between 0 and 2');
  });

  it('should accept optional maxTokens', () => {
    const valid = { ...mockPromptMetadata };
    delete (valid as any).maxTokens;
    expect(() => validatePromptMetadata(valid)).not.toThrow();
  });

  it('should reject invalid maxTokens', () => {
    const invalid = { ...mockPromptMetadata, maxTokens: -1 };
    expect(() => validatePromptMetadata(invalid)).toThrow(ValidationError);
  });

  it('should reject non-array toolsAvailable', () => {
    const invalid = { ...mockPromptMetadata, toolsAvailable: 'not-an-array' };
    expect(() => validatePromptMetadata(invalid)).toThrow(ValidationError);
  });

  it('should reject non-string tools', () => {
    const invalid = { ...mockPromptMetadata, toolsAvailable: ['tool1', 123] };
    expect(() => validatePromptMetadata(invalid)).toThrow(ValidationError);
  });

  it('should reject invalid custom metadata', () => {
    const invalid = { ...mockPromptMetadata, custom: { key: 123 } };
    expect(() => validatePromptMetadata(invalid)).toThrow(ValidationError);
  });
});

describe('validateResponseMetadata', () => {
  it('should accept valid response metadata', () => {
    expect(() => validateResponseMetadata(mockResponseMetadata)).not.toThrow();
  });

  it('should reject invalid model', () => {
    const invalid = { ...mockResponseMetadata, model: '' };
    expect(() => validateResponseMetadata(invalid)).toThrow(ValidationError);
  });

  it('should reject invalid finishReason', () => {
    const invalid = { ...mockResponseMetadata, finishReason: '' };
    expect(() => validateResponseMetadata(invalid)).toThrow(ValidationError);
  });

  it('should reject invalid latencyMs', () => {
    const invalid = { ...mockResponseMetadata, latencyMs: -1 };
    expect(() => validateResponseMetadata(invalid)).toThrow(ValidationError);
  });
});

describe('validateAddPromptRequest', () => {
  it('should accept valid request', () => {
    const request = {
      sessionId: 'session-123',
      content: 'Hello world',
      metadata: mockPromptMetadata,
    };
    expect(() => validateAddPromptRequest(request)).not.toThrow();
  });

  it('should accept request without metadata', () => {
    const request = {
      sessionId: 'session-123',
      content: 'Hello world',
    };
    expect(() => validateAddPromptRequest(request)).not.toThrow();
  });

  it('should reject invalid sessionId', () => {
    const invalid = { sessionId: '', content: 'Hello' };
    expect(() => validateAddPromptRequest(invalid)).toThrow(ValidationError);
  });

  it('should reject invalid content', () => {
    const invalid = { sessionId: 'session-123', content: '' };
    expect(() => validateAddPromptRequest(invalid)).toThrow(ValidationError);
  });

  it('should reject invalid metadata', () => {
    const invalid = {
      sessionId: 'session-123',
      content: 'Hello',
      metadata: { ...mockPromptMetadata, temperature: 5 },
    };
    expect(() => validateAddPromptRequest(invalid)).toThrow(ValidationError);
  });
});

describe('validateAddResponseRequest', () => {
  it('should accept valid request', () => {
    const request = {
      promptId: 'prompt-123',
      content: 'Response',
      tokenUsage: mockTokenUsage,
      metadata: mockResponseMetadata,
    };
    expect(() => validateAddResponseRequest(request)).not.toThrow();
  });

  it('should accept request without metadata', () => {
    const request = {
      promptId: 'prompt-123',
      content: 'Response',
      tokenUsage: mockTokenUsage,
    };
    expect(() => validateAddResponseRequest(request)).not.toThrow();
  });

  it('should reject invalid promptId', () => {
    const invalid = { promptId: '', content: 'Response', tokenUsage: mockTokenUsage };
    expect(() => validateAddResponseRequest(invalid)).toThrow(ValidationError);
  });

  it('should reject invalid content', () => {
    const invalid = { promptId: 'prompt-123', content: '', tokenUsage: mockTokenUsage };
    expect(() => validateAddResponseRequest(invalid)).toThrow(ValidationError);
  });

  it('should reject invalid tokenUsage', () => {
    const invalid = {
      promptId: 'prompt-123',
      content: 'Response',
      tokenUsage: { promptTokens: 10, completionTokens: 20, totalTokens: 25 },
    };
    expect(() => validateAddResponseRequest(invalid)).toThrow(ValidationError);
  });
});

describe('validateQueryOptions', () => {
  it('should accept empty options', () => {
    expect(() => validateQueryOptions({})).not.toThrow();
    expect(() => validateQueryOptions(null)).not.toThrow();
    expect(() => validateQueryOptions(undefined)).not.toThrow();
  });

  it('should accept valid options', () => {
    const options = {
      sessionId: 'session-123',
      nodeType: NodeType.PROMPT,
      limit: 10,
      offset: 0,
      after: new Date('2024-01-01'),
      before: new Date('2024-12-31'),
      filters: { key: 'value' },
    };
    expect(() => validateQueryOptions(options)).not.toThrow();
  });

  it('should reject invalid sessionId', () => {
    expect(() => validateQueryOptions({ sessionId: '' })).toThrow(ValidationError);
  });

  it('should reject invalid nodeType', () => {
    expect(() => validateQueryOptions({ nodeType: 'INVALID' })).toThrow(ValidationError);
  });

  it('should reject invalid limit', () => {
    expect(() => validateQueryOptions({ limit: 0 })).toThrow(ValidationError);
    expect(() => validateQueryOptions({ limit: 20000 })).toThrow(ValidationError);
  });

  it('should reject invalid offset', () => {
    expect(() => validateQueryOptions({ offset: -1 })).toThrow(ValidationError);
  });

  it('should reject invalid dates', () => {
    expect(() => validateQueryOptions({ after: 'not-a-date' })).toThrow(ValidationError);
    expect(() => validateQueryOptions({ before: 'not-a-date' })).toThrow(ValidationError);
  });
});

describe('validateNode', () => {
  it('should accept valid node', () => {
    const node = {
      id: 'node-123',
      type: NodeType.PROMPT,
      createdAt: new Date(),
    };
    expect(() => validateNode(node)).not.toThrow();
  });

  it('should reject invalid id', () => {
    const invalid = { id: '', type: NodeType.PROMPT, createdAt: new Date() };
    expect(() => validateNode(invalid)).toThrow(ValidationError);
  });

  it('should reject invalid type', () => {
    const invalid = { id: 'node-123', type: 'INVALID', createdAt: new Date() };
    expect(() => validateNode(invalid)).toThrow(ValidationError);
  });

  it('should reject invalid createdAt', () => {
    const invalid = { id: 'node-123', type: NodeType.PROMPT, createdAt: 'not-a-date' };
    expect(() => validateNode(invalid)).toThrow(ValidationError);
  });
});

describe('validateEdge', () => {
  it('should accept valid edge', () => {
    const edge = {
      id: 'edge-123',
      fromNodeId: 'node-1',
      toNodeId: 'node-2',
      type: EdgeType.RESPONDS_TO,
      createdAt: new Date(),
      properties: { key: 'value' },
    };
    expect(() => validateEdge(edge)).not.toThrow();
  });

  it('should accept edge without properties', () => {
    const edge = {
      id: 'edge-123',
      fromNodeId: 'node-1',
      toNodeId: 'node-2',
      type: EdgeType.RESPONDS_TO,
      createdAt: new Date(),
    };
    expect(() => validateEdge(edge)).not.toThrow();
  });

  it('should reject invalid properties', () => {
    const invalid = {
      id: 'edge-123',
      fromNodeId: 'node-1',
      toNodeId: 'node-2',
      type: EdgeType.RESPONDS_TO,
      createdAt: new Date(),
      properties: { key: 123 },
    };
    expect(() => validateEdge(invalid)).toThrow(ValidationError);
  });
});

describe('validateEdgeDirection', () => {
  it('should accept valid direction', () => {
    expect(() => validateEdgeDirection(EdgeDirection.INCOMING)).not.toThrow();
    expect(() => validateEdgeDirection(EdgeDirection.OUTGOING)).not.toThrow();
    expect(() => validateEdgeDirection(EdgeDirection.BOTH)).not.toThrow();
  });

  it('should reject invalid direction', () => {
    expect(() => validateEdgeDirection('INVALID')).toThrow(ValidationError);
  });
});

describe('validateEdgeType', () => {
  it('should accept valid type', () => {
    expect(() => validateEdgeType(EdgeType.RESPONDS_TO)).not.toThrow();
    expect(() => validateEdgeType(EdgeType.USES_TOOL)).not.toThrow();
  });

  it('should reject invalid type', () => {
    expect(() => validateEdgeType('INVALID')).toThrow(ValidationError);
  });
});

describe('validateNodeArray', () => {
  it('should accept valid array', () => {
    const nodes = [
      { id: 'node-1', type: NodeType.PROMPT, createdAt: new Date() },
      { id: 'node-2', type: NodeType.RESPONSE, createdAt: new Date() },
    ];
    expect(() => validateNodeArray(nodes)).not.toThrow();
  });

  it('should reject empty array', () => {
    expect(() => validateNodeArray([])).toThrow(ValidationError);
    expect(() => validateNodeArray([])).toThrow('nodes array cannot be empty');
  });

  it('should reject non-array', () => {
    expect(() => validateNodeArray('not-an-array')).toThrow(ValidationError);
  });

  it('should reject array with invalid nodes', () => {
    const invalid = [{ id: '', type: NodeType.PROMPT, createdAt: new Date() }];
    expect(() => validateNodeArray(invalid)).toThrow(ValidationError);
  });
});

describe('validateNodeIdArray', () => {
  it('should accept valid array', () => {
    expect(() => validateNodeIdArray(['node-1', 'node-2'])).not.toThrow();
  });

  it('should reject empty array', () => {
    expect(() => validateNodeIdArray([])).toThrow(ValidationError);
  });

  it('should reject array with invalid IDs', () => {
    expect(() => validateNodeIdArray(['node-1', ''])).toThrow(ValidationError);
  });
});

describe('validateLimit', () => {
  it('should accept valid limits', () => {
    expect(() => validateLimit(1)).not.toThrow();
    expect(() => validateLimit(100)).not.toThrow();
    expect(() => validateLimit(10000)).not.toThrow();
  });

  it('should reject invalid limits', () => {
    expect(() => validateLimit(0)).toThrow(ValidationError);
    expect(() => validateLimit(10001)).toThrow(ValidationError);
  });
});

describe('validateOffset', () => {
  it('should accept valid offsets', () => {
    expect(() => validateOffset(0)).not.toThrow();
    expect(() => validateOffset(100)).not.toThrow();
  });

  it('should reject negative offsets', () => {
    expect(() => validateOffset(-1)).toThrow(ValidationError);
  });
});

describe('sanitizeInput', () => {
  it('should remove script tags', () => {
    const input = 'Hello <script>alert("xss")</script> world';
    const result = sanitizeInput(input);
    expect(result).toBe('Hello  world');
  });

  it('should remove iframe tags', () => {
    const input = 'Hello <iframe src="evil.com"></iframe> world';
    const result = sanitizeInput(input);
    expect(result).toBe('Hello  world');
  });

  it('should remove javascript: protocol', () => {
    const input = 'Hello javascript:alert("xss") world';
    const result = sanitizeInput(input);
    expect(result).toBe('Hello alert("xss") world');
  });

  it('should remove event handlers', () => {
    const input = 'Hello onclick="alert()" world';
    const result = sanitizeInput(input);
    expect(result).toBe('Hello  world');
  });

  it('should not modify safe input', () => {
    const input = 'Hello world, this is safe text';
    const result = sanitizeInput(input);
    expect(result).toBe(input);
  });
});

describe('validateJsonString', () => {
  it('should accept valid JSON', () => {
    expect(() => validateJsonString('{"key": "value"}', 'field')).not.toThrow();
    expect(() => validateJsonString('[]', 'field')).not.toThrow();
  });

  it('should reject invalid JSON', () => {
    expect(() => validateJsonString('{invalid json}', 'field')).toThrow(ValidationError);
    expect(() => validateJsonString('{invalid json}', 'field')).toThrow('field must be valid JSON');
  });

  it('should reject empty string', () => {
    expect(() => validateJsonString('', 'field')).toThrow(ValidationError);
  });
});
