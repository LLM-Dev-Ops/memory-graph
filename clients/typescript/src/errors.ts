/**
 * Error handling module for LLM Memory Graph Client
 *
 * Provides comprehensive error classes with proper error hierarchy
 * and gRPC status code mapping.
 *
 * @module errors
 */

import * as grpc from '@grpc/grpc-js';

/**
 * Base error class for all Memory Graph errors
 */
export class MemoryGraphError extends Error {
  /**
   * Error code for programmatic handling
   */
  public readonly code: string;

  /**
   * HTTP-style status code (optional)
   */
  public readonly statusCode?: number;

  /**
   * Additional error details
   */
  public readonly details?: Record<string, unknown>;

  /**
   * Original error that caused this error
   */
  public readonly cause?: Error;

  constructor(
    message: string,
    code: string,
    statusCode?: number,
    details?: Record<string, unknown>,
    cause?: Error
  ) {
    super(message);
    this.name = this.constructor.name;
    this.code = code;
    this.statusCode = statusCode;
    this.details = details;
    this.cause = cause;

    // Maintains proper stack trace for where error was thrown (V8 only)
    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Convert error to JSON representation
   */
  toJSON(): Record<string, unknown> {
    return {
      name: this.name,
      message: this.message,
      code: this.code,
      statusCode: this.statusCode,
      details: this.details,
      stack: this.stack,
    };
  }
}

/**
 * Error thrown when connection to the server fails
 */
export class ConnectionError extends MemoryGraphError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'CONNECTION_ERROR', 503, details, cause);
  }
}

/**
 * Error thrown when connection times out
 */
export class TimeoutError extends MemoryGraphError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'TIMEOUT_ERROR', 408, details, cause);
  }
}

/**
 * Error thrown when input validation fails
 */
export class ValidationError extends MemoryGraphError {
  /**
   * Field that failed validation
   */
  public readonly field?: string;

  constructor(message: string, field?: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'VALIDATION_ERROR', 400, details, cause);
    this.field = field;
  }
}

/**
 * Error thrown when authentication fails
 */
export class AuthenticationError extends MemoryGraphError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'AUTHENTICATION_ERROR', 401, details, cause);
  }
}

/**
 * Error thrown when authorization fails (insufficient permissions)
 */
export class AuthorizationError extends MemoryGraphError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'AUTHORIZATION_ERROR', 403, details, cause);
  }
}

/**
 * Error thrown when a requested resource is not found
 */
export class NotFoundError extends MemoryGraphError {
  /**
   * Type of resource that was not found
   */
  public readonly resourceType?: string;

  /**
   * ID of the resource that was not found
   */
  public readonly resourceId?: string;

  constructor(
    message: string,
    resourceType?: string,
    resourceId?: string,
    details?: Record<string, unknown>,
    cause?: Error
  ) {
    super(message, 'NOT_FOUND_ERROR', 404, details, cause);
    this.resourceType = resourceType;
    this.resourceId = resourceId;
  }
}

/**
 * Error thrown when a resource already exists
 */
export class AlreadyExistsError extends MemoryGraphError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'ALREADY_EXISTS_ERROR', 409, details, cause);
  }
}

/**
 * Error thrown when rate limit is exceeded
 */
export class RateLimitError extends MemoryGraphError {
  /**
   * When the rate limit will reset (Unix timestamp)
   */
  public readonly resetAt?: number;

  /**
   * Retry after this many seconds
   */
  public readonly retryAfter?: number;

  constructor(
    message: string,
    retryAfter?: number,
    resetAt?: number,
    details?: Record<string, unknown>,
    cause?: Error
  ) {
    super(message, 'RATE_LIMIT_ERROR', 429, details, cause);
    this.retryAfter = retryAfter;
    this.resetAt = resetAt;
  }
}

/**
 * Error thrown when server returns an internal error
 */
export class InternalServerError extends MemoryGraphError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'INTERNAL_SERVER_ERROR', 500, details, cause);
  }
}

/**
 * Error thrown when service is unavailable
 */
export class ServiceUnavailableError extends MemoryGraphError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'SERVICE_UNAVAILABLE_ERROR', 503, details, cause);
  }
}

/**
 * Error thrown when maximum retries are exceeded
 */
export class MaxRetriesExceededError extends MemoryGraphError {
  /**
   * Number of retry attempts made
   */
  public readonly attemptCount: number;

  constructor(
    message: string,
    attemptCount: number,
    details?: Record<string, unknown>,
    cause?: Error
  ) {
    super(message, 'MAX_RETRIES_EXCEEDED', 503, details, cause);
    this.attemptCount = attemptCount;
  }
}

/**
 * Error thrown when operation is cancelled
 */
export class CancelledError extends MemoryGraphError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'CANCELLED_ERROR', 499, details, cause);
  }
}

/**
 * Error thrown when precondition fails
 */
export class PreconditionFailedError extends MemoryGraphError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'PRECONDITION_FAILED_ERROR', 412, details, cause);
  }
}

/**
 * Error thrown when request is aborted
 */
export class AbortedError extends MemoryGraphError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'ABORTED_ERROR', 409, details, cause);
  }
}

/**
 * Error thrown when operation is not implemented
 */
export class NotImplementedError extends MemoryGraphError {
  constructor(message: string, details?: Record<string, unknown>, cause?: Error) {
    super(message, 'NOT_IMPLEMENTED_ERROR', 501, details, cause);
  }
}

/**
 * Map gRPC status codes to MemoryGraphError instances
 *
 * @param error - The gRPC error
 * @returns A specific MemoryGraphError subclass
 *
 * @example
 * ```typescript
 * try {
 *   await grpcCall();
 * } catch (err) {
 *   throw mapGrpcError(err);
 * }
 * ```
 */
export function mapGrpcError(error: any): MemoryGraphError {
  const message = error.message || 'Unknown error';
  const details = error.metadata ? Object.fromEntries(error.metadata.getMap()) : undefined;

  // Handle gRPC status codes
  if (error.code !== undefined) {
    switch (error.code) {
      case grpc.status.CANCELLED:
        return new CancelledError(message, details, error);

      case grpc.status.UNKNOWN:
        return new InternalServerError(message, details, error);

      case grpc.status.INVALID_ARGUMENT:
        return new ValidationError(message, undefined, details, error);

      case grpc.status.DEADLINE_EXCEEDED:
        return new TimeoutError(message, details, error);

      case grpc.status.NOT_FOUND:
        return new NotFoundError(message, undefined, undefined, details, error);

      case grpc.status.ALREADY_EXISTS:
        return new AlreadyExistsError(message, details, error);

      case grpc.status.PERMISSION_DENIED:
        return new AuthorizationError(message, details, error);

      case grpc.status.RESOURCE_EXHAUSTED:
        return new RateLimitError(message, undefined, undefined, details, error);

      case grpc.status.FAILED_PRECONDITION:
        return new PreconditionFailedError(message, details, error);

      case grpc.status.ABORTED:
        return new AbortedError(message, details, error);

      case grpc.status.UNIMPLEMENTED:
        return new NotImplementedError(message, details, error);

      case grpc.status.INTERNAL:
        return new InternalServerError(message, details, error);

      case grpc.status.UNAVAILABLE:
        return new ServiceUnavailableError(message, details, error);

      case grpc.status.UNAUTHENTICATED:
        return new AuthenticationError(message, details, error);

      default:
        return new MemoryGraphError(message, 'UNKNOWN_ERROR', undefined, details, error);
    }
  }

  // Handle connection errors
  if (error.code === 'ECONNREFUSED' || error.code === 'ENOTFOUND' || error.code === 'ECONNRESET') {
    return new ConnectionError(message, { errorCode: error.code }, error);
  }

  // Handle timeout errors
  if (error.code === 'ETIMEDOUT' || error.code === 'ESOCKETTIMEDOUT') {
    return new TimeoutError(message, { errorCode: error.code }, error);
  }

  // Default to generic MemoryGraphError
  return new MemoryGraphError(message, 'UNKNOWN_ERROR', undefined, details, error);
}

/**
 * Type guard to check if error is a MemoryGraphError
 *
 * @param error - Error to check
 * @returns True if error is a MemoryGraphError
 *
 * @example
 * ```typescript
 * try {
 *   await client.getNode('invalid-id');
 * } catch (error) {
 *   if (isMemoryGraphError(error)) {
 *     console.log('Error code:', error.code);
 *   }
 * }
 * ```
 */
export function isMemoryGraphError(error: unknown): error is MemoryGraphError {
  return error instanceof MemoryGraphError;
}

/**
 * Type guard to check if error is retryable
 *
 * @param error - Error to check
 * @returns True if error is retryable
 *
 * @example
 * ```typescript
 * try {
 *   await client.getNode('some-id');
 * } catch (error) {
 *   if (isRetryableError(error)) {
 *     // Retry the operation
 *   }
 * }
 * ```
 */
export function isRetryableError(error: unknown): boolean {
  if (!isMemoryGraphError(error)) {
    return false;
  }

  // These error types are retryable
  return (
    error instanceof ConnectionError ||
    error instanceof TimeoutError ||
    error instanceof ServiceUnavailableError ||
    error instanceof InternalServerError ||
    error instanceof RateLimitError
  );
}
