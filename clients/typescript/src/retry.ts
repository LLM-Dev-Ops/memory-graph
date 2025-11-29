/**
 * Retry logic module with exponential backoff
 *
 * Provides configurable retry mechanisms for handling transient failures
 * with exponential backoff and jitter.
 *
 * @module retry
 */

import { isRetryableError } from './errors';

/**
 * Retry policy configuration
 */
export interface RetryPolicy {
  /**
   * Maximum number of retry attempts (default: 3)
   */
  maxRetries?: number;

  /**
   * Initial backoff delay in milliseconds (default: 100)
   */
  initialBackoff?: number;

  /**
   * Maximum backoff delay in milliseconds (default: 10000)
   */
  maxBackoff?: number;

  /**
   * Backoff multiplier for exponential backoff (default: 2)
   */
  backoffMultiplier?: number;

  /**
   * Whether to add jitter to backoff delays (default: true)
   * Jitter helps prevent thundering herd problem
   */
  useJitter?: boolean;

  /**
   * Custom function to determine if an error is retryable
   * If not provided, uses the default isRetryableError function
   */
  isRetryable?: (error: unknown) => boolean;

  /**
   * Callback invoked before each retry attempt
   */
  onRetry?: (error: unknown, attempt: number, delayMs: number) => void;
}

/**
 * Default retry policy values
 */
export const DEFAULT_RETRY_POLICY: Required<Omit<RetryPolicy, 'onRetry' | 'isRetryable'>> = {
  maxRetries: 3,
  initialBackoff: 100,
  maxBackoff: 10000,
  backoffMultiplier: 2,
  useJitter: true,
};

/**
 * Retry context for tracking retry state
 */
export interface RetryContext {
  /**
   * Current attempt number (1-indexed)
   */
  attempt: number;

  /**
   * Total elapsed time in milliseconds
   */
  elapsedMs: number;

  /**
   * Previous errors encountered
   */
  errors: unknown[];
}

/**
 * Calculate backoff delay with exponential backoff and optional jitter
 *
 * @param attempt - Current attempt number (0-indexed)
 * @param policy - Retry policy configuration
 * @returns Delay in milliseconds
 *
 * @example
 * ```typescript
 * const delay = calculateBackoff(0, { initialBackoff: 100 }); // 100ms
 * const delay2 = calculateBackoff(1, { initialBackoff: 100 }); // ~200ms
 * const delay3 = calculateBackoff(2, { initialBackoff: 100 }); // ~400ms
 * ```
 */
export function calculateBackoff(attempt: number, policy: RetryPolicy): number {
  const {
    initialBackoff = DEFAULT_RETRY_POLICY.initialBackoff,
    maxBackoff = DEFAULT_RETRY_POLICY.maxBackoff,
    backoffMultiplier = DEFAULT_RETRY_POLICY.backoffMultiplier,
    useJitter = DEFAULT_RETRY_POLICY.useJitter,
  } = policy;

  // Calculate exponential backoff
  let delay = initialBackoff * Math.pow(backoffMultiplier, attempt);

  // Cap at max backoff
  delay = Math.min(delay, maxBackoff);

  // Add jitter (random value between 0 and delay)
  if (useJitter) {
    delay = Math.random() * delay;
  }

  return Math.floor(delay);
}

/**
 * Sleep for a specified duration
 *
 * @param ms - Milliseconds to sleep
 * @returns Promise that resolves after the delay
 *
 * @internal
 */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Execute a function with retry logic and exponential backoff
 *
 * @param fn - Async function to execute
 * @param policy - Retry policy configuration
 * @returns Promise with the function result
 * @throws {MaxRetriesExceededError} When all retries are exhausted
 *
 * @example
 * ```typescript
 * // Simple retry
 * const result = await withRetry(
 *   async () => client.getNode('node-id'),
 *   { maxRetries: 3 }
 * );
 *
 * // Custom retry policy with callback
 * const result = await withRetry(
 *   async () => client.createSession(),
 *   {
 *     maxRetries: 5,
 *     initialBackoff: 200,
 *     maxBackoff: 5000,
 *     onRetry: (error, attempt, delay) => {
 *       console.log(`Retry attempt ${attempt} after ${delay}ms`);
 *     }
 *   }
 * );
 *
 * // Custom retry condition
 * const result = await withRetry(
 *   async () => externalApiCall(),
 *   {
 *     maxRetries: 3,
 *     isRetryable: (error) => {
 *       return error instanceof NetworkError && error.code !== 404;
 *     }
 *   }
 * );
 * ```
 */
export async function withRetry<T>(fn: () => Promise<T>, policy: RetryPolicy = {}): Promise<T> {
  const {
    maxRetries = DEFAULT_RETRY_POLICY.maxRetries,
    isRetryable = isRetryableError,
    onRetry,
  } = policy;

  const errors: unknown[] = [];

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      // Execute the function
      return await fn();
    } catch (error) {
      errors.push(error);

      // Check if we should retry
      const shouldRetry = attempt < maxRetries && isRetryable(error);

      if (!shouldRetry) {
        // No more retries, throw the error
        throw error;
      }

      // Calculate backoff delay
      const delayMs = calculateBackoff(attempt, policy);

      // Invoke retry callback if provided
      if (onRetry) {
        onRetry(error, attempt + 1, delayMs);
      }

      // Wait before retrying
      await sleep(delayMs);
    }
  }

  // This should never be reached, but TypeScript needs it
  throw errors[errors.length - 1];
}

/**
 * Create a retry wrapper for a function
 *
 * Returns a new function that automatically retries on failure.
 *
 * @param fn - Function to wrap with retry logic
 * @param policy - Retry policy configuration
 * @returns Wrapped function with retry logic
 *
 * @example
 * ```typescript
 * // Create a retry-enabled version of a method
 * const getNodeWithRetry = withRetryWrapper(
 *   (id: string) => client.getNode(id),
 *   { maxRetries: 3 }
 * );
 *
 * // Use the wrapped function
 * const node = await getNodeWithRetry('node-id');
 * ```
 */
export function withRetryWrapper<TArgs extends unknown[], TResult>(
  fn: (...args: TArgs) => Promise<TResult>,
  policy: RetryPolicy = {}
): (...args: TArgs) => Promise<TResult> {
  return async (...args: TArgs): Promise<TResult> => {
    return withRetry(() => fn(...args), policy);
  };
}

/**
 * Retry manager for advanced retry scenarios
 *
 * Provides fine-grained control over retry logic with context tracking.
 *
 * @example
 * ```typescript
 * const retryManager = new RetryManager({
 *   maxRetries: 5,
 *   initialBackoff: 100,
 *   onRetry: (error, attempt, delay) => {
 *     logger.warn(`Retrying after ${delay}ms (attempt ${attempt})`);
 *   }
 * });
 *
 * const result = await retryManager.execute(async (context) => {
 *   console.log(`Attempt ${context.attempt}`);
 *   return await client.getNode('node-id');
 * });
 * ```
 */
export class RetryManager {
  private readonly policy: RetryPolicy;

  constructor(policy: RetryPolicy = {}) {
    this.policy = policy;
  }

  /**
   * Execute a function with retry logic and context
   *
   * @param fn - Async function to execute, receives retry context
   * @returns Promise with the function result
   */
  async execute<T>(fn: (context: RetryContext) => Promise<T>): Promise<T> {
    const {
      maxRetries = DEFAULT_RETRY_POLICY.maxRetries,
      isRetryable = isRetryableError,
      onRetry,
    } = this.policy;

    const errors: unknown[] = [];
    const startTime = Date.now();

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      const elapsedMs = Date.now() - startTime;
      const context: RetryContext = {
        attempt: attempt + 1,
        elapsedMs,
        errors: [...errors],
      };

      try {
        return await fn(context);
      } catch (error) {
        errors.push(error);

        const shouldRetry = attempt < maxRetries && isRetryable(error);

        if (!shouldRetry) {
          throw error;
        }

        const delayMs = calculateBackoff(attempt, this.policy);

        if (onRetry) {
          onRetry(error, attempt + 1, delayMs);
        }

        await sleep(delayMs);
      }
    }

    throw errors[errors.length - 1];
  }

  /**
   * Update retry policy
   *
   * @param policy - New policy settings to merge
   */
  updatePolicy(policy: Partial<RetryPolicy>): void {
    Object.assign(this.policy, policy);
  }

  /**
   * Get current retry policy
   *
   * @returns Current retry policy
   */
  getPolicy(): Readonly<RetryPolicy> {
    return { ...this.policy };
  }
}

/**
 * Circuit breaker state
 */
export enum CircuitState {
  CLOSED = 'CLOSED',
  OPEN = 'OPEN',
  HALF_OPEN = 'HALF_OPEN',
}

/**
 * Circuit breaker configuration
 */
export interface CircuitBreakerConfig {
  /**
   * Number of failures before opening the circuit
   */
  failureThreshold?: number;

  /**
   * Time window for counting failures in milliseconds
   */
  failureWindow?: number;

  /**
   * Time to wait before attempting to close the circuit in milliseconds
   */
  resetTimeout?: number;

  /**
   * Number of successful calls needed to close the circuit from half-open
   */
  successThreshold?: number;
}

/**
 * Circuit breaker for preventing cascading failures
 *
 * Implements the circuit breaker pattern to fail fast when a service is unhealthy.
 *
 * @example
 * ```typescript
 * const breaker = new CircuitBreaker({
 *   failureThreshold: 5,
 *   resetTimeout: 30000,
 * });
 *
 * try {
 *   const result = await breaker.execute(() => client.getNode('node-id'));
 * } catch (error) {
 *   if (breaker.getState() === CircuitState.OPEN) {
 *     console.log('Circuit is open, service is unavailable');
 *   }
 * }
 * ```
 */
export class CircuitBreaker {
  private state: CircuitState = CircuitState.CLOSED;
  private failureCount = 0;
  private successCount = 0;
  private lastFailureTime = 0;
  private nextAttemptTime = 0;

  private readonly config: Required<CircuitBreakerConfig>;

  constructor(config: CircuitBreakerConfig = {}) {
    this.config = {
      failureThreshold: config.failureThreshold ?? 5,
      failureWindow: config.failureWindow ?? 60000,
      resetTimeout: config.resetTimeout ?? 30000,
      successThreshold: config.successThreshold ?? 2,
    };
  }

  /**
   * Execute a function with circuit breaker protection
   *
   * @param fn - Async function to execute
   * @returns Promise with the function result
   * @throws Error when circuit is open
   */
  async execute<T>(fn: () => Promise<T>): Promise<T> {
    // Check if circuit is open
    if (this.state === CircuitState.OPEN) {
      const now = Date.now();
      if (now < this.nextAttemptTime) {
        throw new Error('Circuit breaker is OPEN');
      }
      // Move to half-open state
      this.state = CircuitState.HALF_OPEN;
      this.successCount = 0;
    }

    try {
      const result = await fn();
      this.onSuccess();
      return result;
    } catch (error) {
      this.onFailure();
      throw error;
    }
  }

  /**
   * Handle successful execution
   */
  private onSuccess(): void {
    this.failureCount = 0;

    if (this.state === CircuitState.HALF_OPEN) {
      this.successCount++;
      if (this.successCount >= this.config.successThreshold) {
        this.state = CircuitState.CLOSED;
        this.successCount = 0;
      }
    }
  }

  /**
   * Handle failed execution
   */
  private onFailure(): void {
    const now = Date.now();
    this.lastFailureTime = now;

    // Reset count if outside failure window
    if (now - this.lastFailureTime > this.config.failureWindow) {
      this.failureCount = 0;
    }

    this.failureCount++;

    if (this.state === CircuitState.HALF_OPEN) {
      // Immediately open on failure in half-open state
      this.state = CircuitState.OPEN;
      this.nextAttemptTime = now + this.config.resetTimeout;
      this.successCount = 0;
    } else if (this.failureCount >= this.config.failureThreshold) {
      // Open circuit if threshold exceeded
      this.state = CircuitState.OPEN;
      this.nextAttemptTime = now + this.config.resetTimeout;
    }
  }

  /**
   * Get current circuit state
   */
  getState(): CircuitState {
    return this.state;
  }

  /**
   * Reset circuit breaker to closed state
   */
  reset(): void {
    this.state = CircuitState.CLOSED;
    this.failureCount = 0;
    this.successCount = 0;
    this.lastFailureTime = 0;
    this.nextAttemptTime = 0;
  }

  /**
   * Get circuit breaker statistics
   */
  getStats(): {
    state: CircuitState;
    failureCount: number;
    successCount: number;
    lastFailureTime: number;
  } {
    return {
      state: this.state,
      failureCount: this.failureCount,
      successCount: this.successCount,
      lastFailureTime: this.lastFailureTime,
    };
  }
}
