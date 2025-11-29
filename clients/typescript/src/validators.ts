/**
 * Input validation module
 *
 * Provides comprehensive validation for all client method inputs
 * with helpful error messages.
 *
 * @module validators
 */

import { ValidationError } from './errors';
import {
  NodeType,
  EdgeType,
  EdgeDirection,
  Node,
  Edge,
  AddPromptRequest,
  AddResponseRequest,
  QueryOptions,
  PromptMetadata,
  ResponseMetadata,
  TokenUsage,
} from './types';

/**
 * Validate that a value is a non-empty string
 *
 * @param value - Value to validate
 * @param fieldName - Name of the field being validated
 * @throws {ValidationError} If validation fails
 *
 * @example
 * ```typescript
 * validateNonEmptyString('hello', 'name'); // OK
 * validateNonEmptyString('', 'name'); // Throws ValidationError
 * validateNonEmptyString(null, 'name'); // Throws ValidationError
 * ```
 */
export function validateNonEmptyString(value: unknown, fieldName: string): asserts value is string {
  if (typeof value !== 'string') {
    throw new ValidationError(`${fieldName} must be a string`, fieldName, {
      received: typeof value,
    });
  }

  if (value.trim().length === 0) {
    throw new ValidationError(`${fieldName} cannot be empty`, fieldName);
  }
}

/**
 * Validate that a value is a valid UUID
 *
 * @param value - Value to validate
 * @param fieldName - Name of the field being validated
 * @throws {ValidationError} If validation fails
 */
export function validateUuid(value: unknown, fieldName: string): asserts value is string {
  validateNonEmptyString(value, fieldName);

  const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  if (!uuidRegex.test(value)) {
    throw new ValidationError(`${fieldName} must be a valid UUID`, fieldName, { value });
  }
}

/**
 * Validate that a value is a positive number
 *
 * @param value - Value to validate
 * @param fieldName - Name of the field being validated
 * @throws {ValidationError} If validation fails
 */
export function validatePositiveNumber(value: unknown, fieldName: string): asserts value is number {
  if (typeof value !== 'number') {
    throw new ValidationError(`${fieldName} must be a number`, fieldName, {
      received: typeof value,
    });
  }

  if (!Number.isFinite(value)) {
    throw new ValidationError(`${fieldName} must be a finite number`, fieldName, { value });
  }

  if (value < 0) {
    throw new ValidationError(`${fieldName} must be positive`, fieldName, { value });
  }
}

/**
 * Validate that a value is a non-negative integer
 *
 * @param value - Value to validate
 * @param fieldName - Name of the field being validated
 * @throws {ValidationError} If validation fails
 */
export function validateNonNegativeInteger(
  value: unknown,
  fieldName: string
): asserts value is number {
  validatePositiveNumber(value, fieldName);

  if (!Number.isInteger(value)) {
    throw new ValidationError(`${fieldName} must be an integer`, fieldName, { value });
  }
}

/**
 * Validate that a value is within a range
 *
 * @param value - Value to validate
 * @param fieldName - Name of the field being validated
 * @param min - Minimum value (inclusive)
 * @param max - Maximum value (inclusive)
 * @throws {ValidationError} If validation fails
 */
export function validateRange(value: number, fieldName: string, min: number, max: number): void {
  if (value < min || value > max) {
    throw new ValidationError(`${fieldName} must be between ${min} and ${max}`, fieldName, {
      value,
      min,
      max,
    });
  }
}

/**
 * Validate that a value is a valid Date
 *
 * @param value - Value to validate
 * @param fieldName - Name of the field being validated
 * @throws {ValidationError} If validation fails
 */
export function validateDate(value: unknown, fieldName: string): asserts value is Date {
  if (!(value instanceof Date)) {
    throw new ValidationError(`${fieldName} must be a Date object`, fieldName, {
      received: typeof value,
    });
  }

  if (isNaN(value.getTime())) {
    throw new ValidationError(`${fieldName} must be a valid Date`, fieldName);
  }
}

/**
 * Validate that a value is a valid object
 *
 * @param value - Value to validate
 * @param fieldName - Name of the field being validated
 * @throws {ValidationError} If validation fails
 */
export function validateObject(
  value: unknown,
  fieldName: string
): asserts value is Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new ValidationError(`${fieldName} must be an object`, fieldName, {
      received: typeof value,
    });
  }
}

/**
 * Validate that a value is a valid enum value
 *
 * @param value - Value to validate
 * @param fieldName - Name of the field being validated
 * @param enumObj - Enum object
 * @throws {ValidationError} If validation fails
 */
export function validateEnum<T extends Record<string, unknown>>(
  value: unknown,
  fieldName: string,
  enumObj: T
): asserts value is T[keyof T] {
  const validValues = Object.values(enumObj);
  if (!validValues.includes(value)) {
    throw new ValidationError(`${fieldName} must be one of: ${validValues.join(', ')}`, fieldName, {
      value,
      validValues,
    });
  }
}

/**
 * Validate session ID
 *
 * @param sessionId - Session ID to validate
 * @throws {ValidationError} If validation fails
 */
export function validateSessionId(sessionId: unknown): asserts sessionId is string {
  validateNonEmptyString(sessionId, 'sessionId');
}

/**
 * Validate node ID
 *
 * @param nodeId - Node ID to validate
 * @throws {ValidationError} If validation fails
 */
export function validateNodeId(nodeId: unknown): asserts nodeId is string {
  validateNonEmptyString(nodeId, 'nodeId');
}

/**
 * Validate edge ID
 *
 * @param edgeId - Edge ID to validate
 * @throws {ValidationError} If validation fails
 */
export function validateEdgeId(edgeId: unknown): asserts edgeId is string {
  validateNonEmptyString(edgeId, 'edgeId');
}

/**
 * Validate metadata object
 *
 * @param metadata - Metadata to validate
 * @param fieldName - Name of the field being validated
 * @throws {ValidationError} If validation fails
 */
export function validateMetadata(
  metadata: unknown,
  fieldName: string
): asserts metadata is Record<string, string> {
  validateObject(metadata, fieldName);

  for (const [key, value] of Object.entries(metadata as Record<string, unknown>)) {
    if (typeof value !== 'string') {
      throw new ValidationError(`${fieldName}.${key} must be a string`, `${fieldName}.${key}`, {
        received: typeof value,
      });
    }
  }
}

/**
 * Validate token usage object
 *
 * @param tokenUsage - Token usage to validate
 * @throws {ValidationError} If validation fails
 */
export function validateTokenUsage(tokenUsage: unknown): asserts tokenUsage is TokenUsage {
  validateObject(tokenUsage, 'tokenUsage');

  const usage = tokenUsage as Record<string, unknown>;

  validateNonNegativeInteger(usage.promptTokens, 'tokenUsage.promptTokens');
  validateNonNegativeInteger(usage.completionTokens, 'tokenUsage.completionTokens');
  validateNonNegativeInteger(usage.totalTokens, 'tokenUsage.totalTokens');

  // Validate that total equals sum
  const total = usage.totalTokens as number;
  const sum = (usage.promptTokens as number) + (usage.completionTokens as number);
  if (total !== sum) {
    throw new ValidationError(
      'tokenUsage.totalTokens must equal promptTokens + completionTokens',
      'tokenUsage.totalTokens',
      { total, sum }
    );
  }
}

/**
 * Validate prompt metadata
 *
 * @param metadata - Prompt metadata to validate
 * @throws {ValidationError} If validation fails
 */
export function validatePromptMetadata(metadata: unknown): asserts metadata is PromptMetadata {
  validateObject(metadata, 'metadata');

  const meta = metadata as Record<string, unknown>;

  validateNonEmptyString(meta.model, 'metadata.model');
  validatePositiveNumber(meta.temperature, 'metadata.temperature');
  validateRange(meta.temperature as number, 'metadata.temperature', 0, 2);

  if (meta.maxTokens !== undefined) {
    validatePositiveNumber(meta.maxTokens, 'metadata.maxTokens');
  }

  if (!Array.isArray(meta.toolsAvailable)) {
    throw new ValidationError(
      'metadata.toolsAvailable must be an array',
      'metadata.toolsAvailable'
    );
  }

  for (let i = 0; i < meta.toolsAvailable.length; i++) {
    const tool = (meta.toolsAvailable as unknown[])[i];
    if (typeof tool !== 'string') {
      throw new ValidationError(
        `metadata.toolsAvailable[${i}] must be a string`,
        `metadata.toolsAvailable[${i}]`
      );
    }
  }

  validateMetadata(meta.custom, 'metadata.custom');
}

/**
 * Validate response metadata
 *
 * @param metadata - Response metadata to validate
 * @throws {ValidationError} If validation fails
 */
export function validateResponseMetadata(metadata: unknown): asserts metadata is ResponseMetadata {
  validateObject(metadata, 'metadata');

  const meta = metadata as Record<string, unknown>;

  validateNonEmptyString(meta.model, 'metadata.model');
  validateNonEmptyString(meta.finishReason, 'metadata.finishReason');
  validateNonNegativeInteger(meta.latencyMs, 'metadata.latencyMs');
  validateMetadata(meta.custom, 'metadata.custom');
}

/**
 * Validate add prompt request
 *
 * @param request - Add prompt request to validate
 * @throws {ValidationError} If validation fails
 */
export function validateAddPromptRequest(request: unknown): asserts request is AddPromptRequest {
  validateObject(request, 'request');

  const req = request as Record<string, unknown>;

  validateSessionId(req.sessionId);
  validateNonEmptyString(req.content, 'content');

  if (req.metadata !== undefined) {
    validatePromptMetadata(req.metadata);
  }
}

/**
 * Validate add response request
 *
 * @param request - Add response request to validate
 * @throws {ValidationError} If validation fails
 */
export function validateAddResponseRequest(
  request: unknown
): asserts request is AddResponseRequest {
  validateObject(request, 'request');

  const req = request as Record<string, unknown>;

  validateNonEmptyString(req.promptId, 'promptId');
  validateNonEmptyString(req.content, 'content');
  validateTokenUsage(req.tokenUsage);

  if (req.metadata !== undefined) {
    validateResponseMetadata(req.metadata);
  }
}

/**
 * Validate query options
 *
 * @param options - Query options to validate
 * @throws {ValidationError} If validation fails
 */
export function validateQueryOptions(options: unknown): asserts options is QueryOptions {
  if (options === null || options === undefined) {
    return;
  }

  validateObject(options, 'options');

  const opts = options as Record<string, unknown>;

  if (opts.sessionId !== undefined) {
    validateSessionId(opts.sessionId);
  }

  if (opts.nodeType !== undefined) {
    validateEnum(opts.nodeType, 'nodeType', NodeType);
  }

  if (opts.after !== undefined) {
    validateDate(opts.after, 'after');
  }

  if (opts.before !== undefined) {
    validateDate(opts.before, 'before');
  }

  if (opts.limit !== undefined) {
    validateNonNegativeInteger(opts.limit, 'limit');
    validateRange(opts.limit as number, 'limit', 1, 10000);
  }

  if (opts.offset !== undefined) {
    validateNonNegativeInteger(opts.offset, 'offset');
  }

  if (opts.filters !== undefined) {
    validateMetadata(opts.filters, 'filters');
  }
}

/**
 * Validate node object
 *
 * @param node - Node to validate
 * @throws {ValidationError} If validation fails
 */
export function validateNode(node: unknown): asserts node is Node {
  validateObject(node, 'node');

  const n = node as Record<string, unknown>;

  validateNonEmptyString(n.id, 'node.id');
  validateEnum(n.type, 'node.type', NodeType);
  validateDate(n.createdAt, 'node.createdAt');
}

/**
 * Validate edge object
 *
 * @param edge - Edge to validate
 * @throws {ValidationError} If validation fails
 */
export function validateEdge(edge: unknown): asserts edge is Edge {
  validateObject(edge, 'edge');

  const e = edge as Record<string, unknown>;

  validateNonEmptyString(e.id, 'edge.id');
  validateNonEmptyString(e.fromNodeId, 'edge.fromNodeId');
  validateNonEmptyString(e.toNodeId, 'edge.toNodeId');
  validateEnum(e.type, 'edge.type', EdgeType);
  validateDate(e.createdAt, 'edge.createdAt');

  if (e.properties !== undefined) {
    validateMetadata(e.properties, 'edge.properties');
  }
}

/**
 * Validate edge direction
 *
 * @param direction - Edge direction to validate
 * @throws {ValidationError} If validation fails
 */
export function validateEdgeDirection(direction: unknown): asserts direction is EdgeDirection {
  validateEnum(direction, 'direction', EdgeDirection);
}

/**
 * Validate edge type
 *
 * @param type - Edge type to validate
 * @throws {ValidationError} If validation fails
 */
export function validateEdgeType(type: unknown): asserts type is EdgeType {
  validateEnum(type, 'type', EdgeType);
}

/**
 * Validate array of nodes
 *
 * @param nodes - Array of nodes to validate
 * @throws {ValidationError} If validation fails
 */
export function validateNodeArray(nodes: unknown): asserts nodes is Node[] {
  if (!Array.isArray(nodes)) {
    throw new ValidationError('nodes must be an array', 'nodes', { received: typeof nodes });
  }

  if (nodes.length === 0) {
    throw new ValidationError('nodes array cannot be empty', 'nodes');
  }

  for (let i = 0; i < nodes.length; i++) {
    try {
      validateNode(nodes[i]);
    } catch (error) {
      if (error instanceof ValidationError) {
        throw new ValidationError(
          `Invalid node at index ${i}: ${error.message}`,
          `nodes[${i}]`,
          error.details,
          error
        );
      }
      throw error;
    }
  }
}

/**
 * Validate array of node IDs
 *
 * @param nodeIds - Array of node IDs to validate
 * @throws {ValidationError} If validation fails
 */
export function validateNodeIdArray(nodeIds: unknown): asserts nodeIds is string[] {
  if (!Array.isArray(nodeIds)) {
    throw new ValidationError('nodeIds must be an array', 'nodeIds', { received: typeof nodeIds });
  }

  if (nodeIds.length === 0) {
    throw new ValidationError('nodeIds array cannot be empty', 'nodeIds');
  }

  for (let i = 0; i < nodeIds.length; i++) {
    try {
      validateNodeId(nodeIds[i]);
    } catch (error) {
      if (error instanceof ValidationError) {
        throw new ValidationError(
          `Invalid node ID at index ${i}: ${error.message}`,
          `nodeIds[${i}]`,
          error.details,
          error
        );
      }
      throw error;
    }
  }
}

/**
 * Validate limit parameter
 *
 * @param limit - Limit to validate
 * @throws {ValidationError} If validation fails
 */
export function validateLimit(limit: unknown): asserts limit is number {
  validateNonNegativeInteger(limit, 'limit');
  validateRange(limit, 'limit', 1, 10000);
}

/**
 * Validate offset parameter
 *
 * @param offset - Offset to validate
 * @throws {ValidationError} If validation fails
 */
export function validateOffset(offset: unknown): asserts offset is number {
  validateNonNegativeInteger(offset, 'offset');
}

/**
 * Sanitize user input by removing dangerous characters
 *
 * @param input - Input to sanitize
 * @returns Sanitized input
 *
 * @example
 * ```typescript
 * const safe = sanitizeInput('Hello <script>alert("xss")</script>');
 * // Returns: 'Hello alert("xss")'
 * ```
 */
export function sanitizeInput(input: string): string {
  // Remove potential XSS patterns (basic sanitization)
  return input
    .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '')
    .replace(/<iframe\b[^<]*(?:(?!<\/iframe>)<[^<]*)*<\/iframe>/gi, '')
    .replace(/javascript:/gi, '')
    .replace(/on\w+\s*=/gi, '');
}

/**
 * Validate JSON string
 *
 * @param value - Value to validate
 * @param fieldName - Name of the field being validated
 * @throws {ValidationError} If validation fails
 */
export function validateJsonString(value: unknown, fieldName: string): asserts value is string {
  validateNonEmptyString(value, fieldName);

  try {
    JSON.parse(value);
  } catch (error) {
    throw new ValidationError(
      `${fieldName} must be valid JSON`,
      fieldName,
      { value },
      error as Error
    );
  }
}
