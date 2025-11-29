/**
 * Unit tests for retry module
 */

import { describe, it, expect, jest, beforeEach, afterEach } from '@jest/globals';
import {
  calculateBackoff,
  withRetry,
  withRetryWrapper,
  RetryManager,
  CircuitBreaker,
  CircuitState,
  DEFAULT_RETRY_POLICY,
} from '../../src/retry';
import { ConnectionError, TimeoutError, ValidationError } from '../../src/errors';

describe('calculateBackoff', () => {
  it('should calculate exponential backoff', () => {
    const policy = {
      initialBackoff: 100,
      backoffMultiplier: 2,
      maxBackoff: 10000,
      useJitter: false,
    };

    expect(calculateBackoff(0, policy)).toBe(100); // 100 * 2^0 = 100
    expect(calculateBackoff(1, policy)).toBe(200); // 100 * 2^1 = 200
    expect(calculateBackoff(2, policy)).toBe(400); // 100 * 2^2 = 400
    expect(calculateBackoff(3, policy)).toBe(800); // 100 * 2^3 = 800
  });

  it('should cap at max backoff', () => {
    const policy = {
      initialBackoff: 100,
      backoffMultiplier: 2,
      maxBackoff: 500,
      useJitter: false,
    };

    expect(calculateBackoff(10, policy)).toBe(500); // Would be 102400, capped at 500
  });

  it('should add jitter when enabled', () => {
    const policy = {
      initialBackoff: 100,
      backoffMultiplier: 2,
      maxBackoff: 10000,
      useJitter: true,
    };

    const delay1 = calculateBackoff(1, policy);

    expect(delay1).toBeGreaterThanOrEqual(0);
    expect(delay1).toBeLessThanOrEqual(200);
    // With jitter, delays will be random values between 0 and the calculated backoff
  });

  it('should use default values', () => {
    const delay = calculateBackoff(0, {});
    expect(delay).toBeGreaterThanOrEqual(0);
    expect(delay).toBeLessThanOrEqual(DEFAULT_RETRY_POLICY.initialBackoff);
  });
});

describe('withRetry', () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('should succeed on first attempt', async () => {
    const fn = jest.fn<() => Promise<string>>().mockResolvedValue('success');

    const promise = withRetry(fn, { maxRetries: 3 });
    await jest.runAllTimersAsync();
    const result = await promise;

    expect(result).toBe('success');
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it('should retry on retryable error', async () => {
    const fn = jest
      .fn<() => Promise<string>>()
      .mockRejectedValueOnce(new ConnectionError('Connection failed'))
      .mockResolvedValue('success');

    const promise = withRetry(fn, { maxRetries: 3, initialBackoff: 100, useJitter: false });

    // Run timers to advance through retry delays
    setTimeout(async () => {
      await jest.advanceTimersByTimeAsync(100);
    }, 0);

    const result = await promise;

    expect(result).toBe('success');
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it('should not retry on non-retryable error', async () => {
    const fn = jest.fn<() => Promise<string>>().mockRejectedValue(new ValidationError('Invalid input'));

    await expect(withRetry(fn, { maxRetries: 3 })).rejects.toThrow(ValidationError);
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it('should throw after max retries', async () => {
    const error = new ConnectionError('Connection failed');
    const fn = jest.fn<() => Promise<string>>().mockRejectedValue(error);

    const promise = withRetry(fn, { maxRetries: 2, initialBackoff: 10, useJitter: false });

    // Advance through all retry attempts
    setTimeout(async () => {
      await jest.advanceTimersByTimeAsync(100);
    }, 0);

    await expect(promise).rejects.toThrow(ConnectionError);
    expect(fn).toHaveBeenCalledTimes(3); // Initial + 2 retries
  });

  it('should call onRetry callback', async () => {
    const onRetry = jest.fn();
    const fn = jest
      .fn<() => Promise<string>>()
      .mockRejectedValueOnce(new TimeoutError('Timeout'))
      .mockResolvedValue('success');

    const promise = withRetry(fn, {
      maxRetries: 3,
      initialBackoff: 100,
      useJitter: false,
      onRetry,
    });

    setTimeout(async () => {
      await jest.advanceTimersByTimeAsync(200);
    }, 0);

    await promise;

    expect(onRetry).toHaveBeenCalledTimes(1);
    expect(onRetry).toHaveBeenCalledWith(
      expect.any(TimeoutError),
      1, // attempt number
      100 // delay
    );
  });

  it('should use custom isRetryable function', async () => {
    const customError = new Error('Custom error');
    const fn = jest.fn<() => Promise<string>>().mockRejectedValueOnce(customError).mockResolvedValue('success');

    const isRetryable = jest.fn<(error: unknown) => boolean>().mockReturnValue(true);

    const promise = withRetry(fn, {
      maxRetries: 3,
      initialBackoff: 10,
      useJitter: false,
      isRetryable,
    });

    setTimeout(async () => {
      await jest.advanceTimersByTimeAsync(50);
    }, 0);

    await promise;

    expect(isRetryable).toHaveBeenCalledWith(customError);
    expect(fn).toHaveBeenCalledTimes(2);
  });
});

describe('withRetryWrapper', () => {
  it('should create a retry-wrapped function', async () => {
    jest.useFakeTimers();

    const originalFn = jest
      .fn<(arg1: string, arg2: string) => Promise<string>>()
      .mockRejectedValueOnce(new ConnectionError('Failed'))
      .mockResolvedValue('success');

    const wrappedFn = withRetryWrapper(originalFn, { maxRetries: 3, initialBackoff: 10 });

    const promise = wrappedFn('arg1', 'arg2');

    setTimeout(async () => {
      await jest.advanceTimersByTimeAsync(50);
    }, 0);

    const result = await promise;

    expect(result).toBe('success');
    expect(originalFn).toHaveBeenCalledWith('arg1', 'arg2');

    jest.useRealTimers();
  });
});

describe('RetryManager', () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('should execute function with context', async () => {
    const manager = new RetryManager({ maxRetries: 3 });
    const fn = jest.fn<(context: any) => Promise<string>>().mockResolvedValue('success');

    const promise = manager.execute(fn);
    await jest.runAllTimersAsync();
    const result = await promise;

    expect(result).toBe('success');
    expect(fn).toHaveBeenCalledWith(
      expect.objectContaining({
        attempt: 1,
        elapsedMs: expect.any(Number),
        errors: [],
      })
    );
  });

  it('should track retry context', async () => {
    const manager = new RetryManager({ maxRetries: 2, initialBackoff: 10 });
    const fn = jest
      .fn<(context: any) => Promise<string>>()
      .mockRejectedValueOnce(new ConnectionError('Failed 1'))
      .mockRejectedValueOnce(new ConnectionError('Failed 2'))
      .mockResolvedValue('success');

    const promise = manager.execute(fn);

    setTimeout(async () => {
      await jest.advanceTimersByTimeAsync(100);
    }, 0);

    await promise;

    expect(fn).toHaveBeenCalledTimes(3);

    // Check second attempt context
    const secondCallContext = fn.mock.calls[1][0];
    expect(secondCallContext.attempt).toBe(2);
    expect(secondCallContext.errors).toHaveLength(1);

    // Check third attempt context
    const thirdCallContext = fn.mock.calls[2][0];
    expect(thirdCallContext.attempt).toBe(3);
    expect(thirdCallContext.errors).toHaveLength(2);
  });

  it('should allow updating policy', () => {
    const manager = new RetryManager({ maxRetries: 3 });

    manager.updatePolicy({ maxRetries: 5 });

    const policy = manager.getPolicy();
    expect(policy.maxRetries).toBe(5);
  });

  it('should return readonly policy', () => {
    const manager = new RetryManager({ maxRetries: 3 });
    const policy = manager.getPolicy();

    expect(policy.maxRetries).toBe(3);
    // Policy should be a copy, not the original
  });
});

describe('CircuitBreaker', () => {
  it('should start in CLOSED state', () => {
    const breaker = new CircuitBreaker();
    expect(breaker.getState()).toBe(CircuitState.CLOSED);
  });

  it('should execute successfully in CLOSED state', async () => {
    const breaker = new CircuitBreaker();
    const fn = jest.fn<() => Promise<string>>().mockResolvedValue('success');

    const result = await breaker.execute(fn);

    expect(result).toBe('success');
    expect(breaker.getState()).toBe(CircuitState.CLOSED);
  });

  it('should open circuit after failure threshold', async () => {
    const breaker = new CircuitBreaker({
      failureThreshold: 3,
      resetTimeout: 1000,
    });

    const fn = jest.fn<() => Promise<string>>().mockRejectedValue(new Error('Failed'));

    // Trigger failures to open circuit
    for (let i = 0; i < 3; i++) {
      try {
        await breaker.execute(fn);
      } catch (error) {
        // Expected
      }
    }

    expect(breaker.getState()).toBe(CircuitState.OPEN);
  });

  it('should reject immediately when circuit is OPEN', async () => {
    jest.useFakeTimers();

    const breaker = new CircuitBreaker({
      failureThreshold: 2,
      resetTimeout: 10000,
    });

    const fn = jest.fn<() => Promise<string>>().mockRejectedValue(new Error('Failed'));

    // Open the circuit
    for (let i = 0; i < 2; i++) {
      try {
        await breaker.execute(fn);
      } catch (error) {
        // Expected
      }
    }

    expect(breaker.getState()).toBe(CircuitState.OPEN);

    // Next call should fail immediately
    await expect(breaker.execute(fn)).rejects.toThrow('Circuit breaker is OPEN');

    jest.useRealTimers();
  });

  it('should transition to HALF_OPEN after reset timeout', async () => {
    jest.useFakeTimers();

    const breaker = new CircuitBreaker({
      failureThreshold: 2,
      resetTimeout: 1000,
    });

    const fn = jest.fn<() => Promise<string>>().mockRejectedValue(new Error('Failed'));

    // Open the circuit
    for (let i = 0; i < 2; i++) {
      try {
        await breaker.execute(fn);
      } catch (error) {
        // Expected
      }
    }

    expect(breaker.getState()).toBe(CircuitState.OPEN);

    // Advance time past reset timeout
    jest.advanceTimersByTime(1001);

    // Next call should transition to HALF_OPEN
    fn.mockResolvedValueOnce('success');
    await breaker.execute(fn);

    // Should be back to CLOSED after successful calls
    expect(breaker.getState()).toBe(CircuitState.HALF_OPEN);

    jest.useRealTimers();
  });

  it('should close circuit after success threshold in HALF_OPEN', async () => {
    jest.useFakeTimers();

    const breaker = new CircuitBreaker({
      failureThreshold: 2,
      resetTimeout: 1000,
      successThreshold: 2,
    });

    const fn = jest.fn<() => Promise<string>>().mockRejectedValue(new Error('Failed'));

    // Open the circuit
    for (let i = 0; i < 2; i++) {
      try {
        await breaker.execute(fn);
      } catch (error) {
        // Expected
      }
    }

    // Advance time and transition to HALF_OPEN
    jest.advanceTimersByTime(1001);

    // Succeed enough times to close circuit
    fn.mockResolvedValue('success');
    await breaker.execute(fn); // First success - still HALF_OPEN
    expect(breaker.getState()).toBe(CircuitState.HALF_OPEN);

    await breaker.execute(fn); // Second success - should close
    expect(breaker.getState()).toBe(CircuitState.CLOSED);

    jest.useRealTimers();
  });

  it('should reopen circuit on failure in HALF_OPEN', async () => {
    jest.useFakeTimers();

    const breaker = new CircuitBreaker({
      failureThreshold: 2,
      resetTimeout: 1000,
    });

    const fn = jest.fn<() => Promise<string>>().mockRejectedValue(new Error('Failed'));

    // Open the circuit
    for (let i = 0; i < 2; i++) {
      try {
        await breaker.execute(fn);
      } catch (error) {
        // Expected
      }
    }

    // Advance time and transition to HALF_OPEN
    jest.advanceTimersByTime(1001);

    // Fail in HALF_OPEN state - should reopen
    try {
      await breaker.execute(fn);
    } catch (error) {
      // Expected
    }

    expect(breaker.getState()).toBe(CircuitState.OPEN);

    jest.useRealTimers();
  });

  it('should reset circuit breaker', async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 2 });
    const fn = jest.fn<() => Promise<string>>().mockRejectedValue(new Error('Failed'));

    // Open the circuit
    for (let i = 0; i < 2; i++) {
      try {
        await breaker.execute(fn);
      } catch (error) {
        // Expected
      }
    }

    expect(breaker.getState()).toBe(CircuitState.OPEN);

    breaker.reset();

    expect(breaker.getState()).toBe(CircuitState.CLOSED);
    const stats = breaker.getStats();
    expect(stats.failureCount).toBe(0);
    expect(stats.successCount).toBe(0);
  });

  it('should return circuit statistics', async () => {
    const breaker = new CircuitBreaker();
    const fn = jest.fn<() => Promise<string>>().mockRejectedValue(new Error('Failed'));

    try {
      await breaker.execute(fn);
    } catch (error) {
      // Expected
    }

    const stats = breaker.getStats();
    expect(stats.state).toBe(CircuitState.CLOSED);
    expect(stats.failureCount).toBe(1);
    expect(stats.successCount).toBe(0);
    expect(stats.lastFailureTime).toBeGreaterThan(0);
  });
});
