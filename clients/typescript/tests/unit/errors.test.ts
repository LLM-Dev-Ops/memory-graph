/**
 * Unit tests for errors module
 */

import { describe, it, expect } from '@jest/globals';
import * as grpc from '@grpc/grpc-js';
import {
  MemoryGraphError,
  ConnectionError,
  TimeoutError,
  ValidationError,
  AuthenticationError,
  AuthorizationError,
  NotFoundError,
  AlreadyExistsError,
  RateLimitError,
  InternalServerError,
  ServiceUnavailableError,
  MaxRetriesExceededError,
  CancelledError,
  PreconditionFailedError,
  AbortedError,
  NotImplementedError,
  mapGrpcError,
  isMemoryGraphError,
  isRetryableError,
} from '../../src/errors';
import { createMockGrpcError } from '../fixtures/mock-data';

describe('MemoryGraphError', () => {
  it('should create error with all properties', () => {
    const error = new MemoryGraphError(
      'Test message',
      'TEST_CODE',
      500,
      { key: 'value' },
      new Error('Cause')
    );

    expect(error.message).toBe('Test message');
    expect(error.code).toBe('TEST_CODE');
    expect(error.statusCode).toBe(500);
    expect(error.details).toEqual({ key: 'value' });
    expect(error.cause).toBeInstanceOf(Error);
    expect(error.name).toBe('MemoryGraphError');
  });

  it('should capture stack trace', () => {
    const error = new MemoryGraphError('Test', 'TEST_CODE');
    expect(error.stack).toBeDefined();
    expect(error.stack).toContain('MemoryGraphError');
  });

  it('should convert to JSON', () => {
    const error = new MemoryGraphError('Test message', 'TEST_CODE', 500, { key: 'value' });
    const json = error.toJSON();

    expect(json.name).toBe('MemoryGraphError');
    expect(json.message).toBe('Test message');
    expect(json.code).toBe('TEST_CODE');
    expect(json.statusCode).toBe(500);
    expect(json.details).toEqual({ key: 'value' });
    expect(json.stack).toBeDefined();
  });
});

describe('Error Classes', () => {
  describe('ConnectionError', () => {
    it('should create with correct properties', () => {
      const error = new ConnectionError('Connection failed', { address: 'localhost:50051' });
      expect(error.message).toBe('Connection failed');
      expect(error.code).toBe('CONNECTION_ERROR');
      expect(error.statusCode).toBe(503);
      expect(error.details).toEqual({ address: 'localhost:50051' });
    });
  });

  describe('TimeoutError', () => {
    it('should create with correct properties', () => {
      const error = new TimeoutError('Request timed out', { timeout: 5000 });
      expect(error.message).toBe('Request timed out');
      expect(error.code).toBe('TIMEOUT_ERROR');
      expect(error.statusCode).toBe(408);
      expect(error.details).toEqual({ timeout: 5000 });
    });
  });

  describe('ValidationError', () => {
    it('should create with field name', () => {
      const error = new ValidationError('Invalid input', 'sessionId', { value: '' });
      expect(error.message).toBe('Invalid input');
      expect(error.code).toBe('VALIDATION_ERROR');
      expect(error.statusCode).toBe(400);
      expect(error.field).toBe('sessionId');
      expect(error.details).toEqual({ value: '' });
    });
  });

  describe('AuthenticationError', () => {
    it('should create with correct properties', () => {
      const error = new AuthenticationError('Auth failed');
      expect(error.code).toBe('AUTHENTICATION_ERROR');
      expect(error.statusCode).toBe(401);
    });
  });

  describe('AuthorizationError', () => {
    it('should create with correct properties', () => {
      const error = new AuthorizationError('Forbidden');
      expect(error.code).toBe('AUTHORIZATION_ERROR');
      expect(error.statusCode).toBe(403);
    });
  });

  describe('NotFoundError', () => {
    it('should create with resource type and ID', () => {
      const error = new NotFoundError('Not found', 'session', 'session-123');
      expect(error.message).toBe('Not found');
      expect(error.code).toBe('NOT_FOUND_ERROR');
      expect(error.statusCode).toBe(404);
      expect(error.resourceType).toBe('session');
      expect(error.resourceId).toBe('session-123');
    });
  });

  describe('AlreadyExistsError', () => {
    it('should create with correct properties', () => {
      const error = new AlreadyExistsError('Already exists');
      expect(error.code).toBe('ALREADY_EXISTS_ERROR');
      expect(error.statusCode).toBe(409);
    });
  });

  describe('RateLimitError', () => {
    it('should create with retry information', () => {
      const error = new RateLimitError('Rate limit exceeded', 60, 1234567890);
      expect(error.message).toBe('Rate limit exceeded');
      expect(error.code).toBe('RATE_LIMIT_ERROR');
      expect(error.statusCode).toBe(429);
      expect(error.retryAfter).toBe(60);
      expect(error.resetAt).toBe(1234567890);
    });
  });

  describe('InternalServerError', () => {
    it('should create with correct properties', () => {
      const error = new InternalServerError('Internal error');
      expect(error.code).toBe('INTERNAL_SERVER_ERROR');
      expect(error.statusCode).toBe(500);
    });
  });

  describe('ServiceUnavailableError', () => {
    it('should create with correct properties', () => {
      const error = new ServiceUnavailableError('Service unavailable');
      expect(error.code).toBe('SERVICE_UNAVAILABLE_ERROR');
      expect(error.statusCode).toBe(503);
    });
  });

  describe('MaxRetriesExceededError', () => {
    it('should create with attempt count', () => {
      const error = new MaxRetriesExceededError('Max retries exceeded', 5);
      expect(error.message).toBe('Max retries exceeded');
      expect(error.code).toBe('MAX_RETRIES_EXCEEDED');
      expect(error.statusCode).toBe(503);
      expect(error.attemptCount).toBe(5);
    });
  });

  describe('CancelledError', () => {
    it('should create with correct properties', () => {
      const error = new CancelledError('Cancelled');
      expect(error.code).toBe('CANCELLED_ERROR');
      expect(error.statusCode).toBe(499);
    });
  });

  describe('PreconditionFailedError', () => {
    it('should create with correct properties', () => {
      const error = new PreconditionFailedError('Precondition failed');
      expect(error.code).toBe('PRECONDITION_FAILED_ERROR');
      expect(error.statusCode).toBe(412);
    });
  });

  describe('AbortedError', () => {
    it('should create with correct properties', () => {
      const error = new AbortedError('Aborted');
      expect(error.code).toBe('ABORTED_ERROR');
      expect(error.statusCode).toBe(409);
    });
  });

  describe('NotImplementedError', () => {
    it('should create with correct properties', () => {
      const error = new NotImplementedError('Not implemented');
      expect(error.code).toBe('NOT_IMPLEMENTED_ERROR');
      expect(error.statusCode).toBe(501);
    });
  });
});

describe('mapGrpcError', () => {
  it('should map CANCELLED status', () => {
    const grpcError = createMockGrpcError(grpc.status.CANCELLED, 'Cancelled');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(CancelledError);
    expect(error.message).toBe('Cancelled');
  });

  it('should map UNKNOWN status', () => {
    const grpcError = createMockGrpcError(grpc.status.UNKNOWN, 'Unknown error');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(InternalServerError);
  });

  it('should map INVALID_ARGUMENT status', () => {
    const grpcError = createMockGrpcError(grpc.status.INVALID_ARGUMENT, 'Invalid argument');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(ValidationError);
  });

  it('should map DEADLINE_EXCEEDED status', () => {
    const grpcError = createMockGrpcError(grpc.status.DEADLINE_EXCEEDED, 'Deadline exceeded');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(TimeoutError);
  });

  it('should map NOT_FOUND status', () => {
    const grpcError = createMockGrpcError(grpc.status.NOT_FOUND, 'Not found');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(NotFoundError);
  });

  it('should map ALREADY_EXISTS status', () => {
    const grpcError = createMockGrpcError(grpc.status.ALREADY_EXISTS, 'Already exists');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(AlreadyExistsError);
  });

  it('should map PERMISSION_DENIED status', () => {
    const grpcError = createMockGrpcError(grpc.status.PERMISSION_DENIED, 'Permission denied');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(AuthorizationError);
  });

  it('should map RESOURCE_EXHAUSTED status', () => {
    const grpcError = createMockGrpcError(grpc.status.RESOURCE_EXHAUSTED, 'Rate limit');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(RateLimitError);
  });

  it('should map FAILED_PRECONDITION status', () => {
    const grpcError = createMockGrpcError(grpc.status.FAILED_PRECONDITION, 'Precondition failed');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(PreconditionFailedError);
  });

  it('should map ABORTED status', () => {
    const grpcError = createMockGrpcError(grpc.status.ABORTED, 'Aborted');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(AbortedError);
  });

  it('should map UNIMPLEMENTED status', () => {
    const grpcError = createMockGrpcError(grpc.status.UNIMPLEMENTED, 'Not implemented');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(NotImplementedError);
  });

  it('should map INTERNAL status', () => {
    const grpcError = createMockGrpcError(grpc.status.INTERNAL, 'Internal error');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(InternalServerError);
  });

  it('should map UNAVAILABLE status', () => {
    const grpcError = createMockGrpcError(grpc.status.UNAVAILABLE, 'Unavailable');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(ServiceUnavailableError);
  });

  it('should map UNAUTHENTICATED status', () => {
    const grpcError = createMockGrpcError(grpc.status.UNAUTHENTICATED, 'Unauthenticated');
    const error = mapGrpcError(grpcError);
    expect(error).toBeInstanceOf(AuthenticationError);
  });

  it('should handle metadata', () => {
    const grpcError = createMockGrpcError(grpc.status.NOT_FOUND, 'Not found', { key: 'value' });
    const error = mapGrpcError(grpcError);
    expect(error.details).toEqual({ key: 'value' });
  });

  it('should handle connection refused errors', () => {
    const error: any = new Error('Connection refused');
    error.code = 'ECONNREFUSED';
    const mappedError = mapGrpcError(error);
    expect(mappedError).toBeInstanceOf(ConnectionError);
    expect(mappedError.details?.errorCode).toBe('ECONNREFUSED');
  });

  it('should handle timeout errors', () => {
    const error: any = new Error('Timeout');
    error.code = 'ETIMEDOUT';
    const mappedError = mapGrpcError(error);
    expect(mappedError).toBeInstanceOf(TimeoutError);
    expect(mappedError.details?.errorCode).toBe('ETIMEDOUT');
  });

  it('should handle unknown errors', () => {
    const error = new Error('Unknown error');
    const mappedError = mapGrpcError(error);
    expect(mappedError).toBeInstanceOf(MemoryGraphError);
    expect(mappedError.code).toBe('UNKNOWN_ERROR');
  });
});

describe('Type Guards', () => {
  describe('isMemoryGraphError', () => {
    it('should return true for MemoryGraphError instances', () => {
      const error = new MemoryGraphError('Test', 'TEST_CODE');
      expect(isMemoryGraphError(error)).toBe(true);
    });

    it('should return true for subclass instances', () => {
      const error = new ValidationError('Invalid', 'field');
      expect(isMemoryGraphError(error)).toBe(true);
    });

    it('should return false for non-MemoryGraphError instances', () => {
      const error = new Error('Standard error');
      expect(isMemoryGraphError(error)).toBe(false);
    });

    it('should return false for non-error values', () => {
      expect(isMemoryGraphError('string')).toBe(false);
      expect(isMemoryGraphError(null)).toBe(false);
      expect(isMemoryGraphError(undefined)).toBe(false);
      expect(isMemoryGraphError({})).toBe(false);
    });
  });

  describe('isRetryableError', () => {
    it('should return true for ConnectionError', () => {
      const error = new ConnectionError('Connection failed');
      expect(isRetryableError(error)).toBe(true);
    });

    it('should return true for TimeoutError', () => {
      const error = new TimeoutError('Timeout');
      expect(isRetryableError(error)).toBe(true);
    });

    it('should return true for ServiceUnavailableError', () => {
      const error = new ServiceUnavailableError('Unavailable');
      expect(isRetryableError(error)).toBe(true);
    });

    it('should return true for InternalServerError', () => {
      const error = new InternalServerError('Internal error');
      expect(isRetryableError(error)).toBe(true);
    });

    it('should return true for RateLimitError', () => {
      const error = new RateLimitError('Rate limit');
      expect(isRetryableError(error)).toBe(true);
    });

    it('should return false for ValidationError', () => {
      const error = new ValidationError('Invalid');
      expect(isRetryableError(error)).toBe(false);
    });

    it('should return false for NotFoundError', () => {
      const error = new NotFoundError('Not found');
      expect(isRetryableError(error)).toBe(false);
    });

    it('should return false for non-MemoryGraphError', () => {
      const error = new Error('Standard error');
      expect(isRetryableError(error)).toBe(false);
    });
  });
});
