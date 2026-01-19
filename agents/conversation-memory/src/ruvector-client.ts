/**
 * RuVector Service Client
 *
 * Client for persisting DecisionEvents to ruvector-service.
 * This is the ONLY interface for physical persistence.
 *
 * CRITICAL: LLM-Memory-Graph NEVER connects directly to Google SQL.
 * All persistence occurs via ruvector-service client calls only.
 */

import type { DecisionEvent, AgentError } from './types.js';

/**
 * RuVector service configuration
 */
export interface RuVectorConfig {
  /**
   * Base URL for ruvector-service
   */
  baseUrl: string;

  /**
   * API key for authentication
   */
  apiKey: string;

  /**
   * Request timeout in milliseconds
   */
  timeoutMs: number;

  /**
   * Maximum retry attempts
   */
  maxRetries: number;

  /**
   * Initial retry delay in milliseconds
   */
  retryDelayMs: number;
}

/**
 * Default configuration
 */
export const DEFAULT_RUVECTOR_CONFIG: RuVectorConfig = {
  baseUrl: process.env['RUVECTOR_SERVICE_URL'] ?? 'http://localhost:8080',
  apiKey: process.env['RUVECTOR_API_KEY'] ?? '',
  timeoutMs: 30000,
  maxRetries: 3,
  retryDelayMs: 1000,
};

/**
 * Response from ruvector-service
 */
export interface RuVectorResponse {
  success: boolean;
  event_id?: string;
  error?: string;
  latency_ms?: number;
}

/**
 * RuVector service client for DecisionEvent persistence
 */
export class RuVectorClient {
  private readonly config: RuVectorConfig;

  constructor(config: Partial<RuVectorConfig> = {}) {
    this.config = { ...DEFAULT_RUVECTOR_CONFIG, ...config };
  }

  /**
   * Persist a DecisionEvent to ruvector-service
   *
   * This is the ONLY method for physical persistence.
   * Every agent invocation MUST emit exactly ONE DecisionEvent.
   */
  async persistDecisionEvent(event: DecisionEvent): Promise<RuVectorResponse> {
    const startTime = Date.now();
    let lastError: Error | null = null;

    for (let attempt = 0; attempt < this.config.maxRetries; attempt++) {
      try {
        const response = await this.makeRequest('/api/v1/decision-events', event);
        return {
          success: true,
          event_id: response.event_id,
          latency_ms: Date.now() - startTime,
        };
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));

        // Don't retry on validation errors
        if (this.isValidationError(lastError)) {
          break;
        }

        // Wait before retry with exponential backoff
        if (attempt < this.config.maxRetries - 1) {
          const delay = this.config.retryDelayMs * Math.pow(2, attempt);
          await this.sleep(delay);
        }
      }
    }

    return {
      success: false,
      error: lastError?.message ?? 'Unknown error',
      latency_ms: Date.now() - startTime,
    };
  }

  /**
   * Retrieve a DecisionEvent by execution_ref
   */
  async retrieveDecisionEvent(executionRef: string): Promise<DecisionEvent | null> {
    try {
      const response = await this.makeRequest(
        `/api/v1/decision-events/${executionRef}`,
        undefined,
        'GET'
      );
      return response as DecisionEvent;
    } catch {
      return null;
    }
  }

  /**
   * Query DecisionEvents by criteria
   */
  async queryDecisionEvents(query: {
    agent_id?: string;
    decision_type?: string;
    session_id?: string;
    from_timestamp?: string;
    to_timestamp?: string;
    limit?: number;
  }): Promise<DecisionEvent[]> {
    try {
      const params = new URLSearchParams();
      for (const [key, value] of Object.entries(query)) {
        if (value !== undefined) {
          params.set(key, String(value));
        }
      }
      const response = await this.makeRequest(
        `/api/v1/decision-events?${params.toString()}`,
        undefined,
        'GET'
      );
      return response.events ?? [];
    } catch {
      return [];
    }
  }

  /**
   * Health check for ruvector-service
   */
  async healthCheck(): Promise<boolean> {
    try {
      await this.makeRequest('/health', undefined, 'GET');
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Make HTTP request to ruvector-service
   */
  private async makeRequest(
    path: string,
    body?: unknown,
    method: 'GET' | 'POST' = 'POST'
  ): Promise<Record<string, unknown>> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeoutMs);

    try {
      const response = await fetch(`${this.config.baseUrl}${path}`, {
        method,
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${this.config.apiKey}`,
          'X-Agent-Id': 'conversation-memory-agent',
        },
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });

      if (!response.ok) {
        const errorBody = await response.text();
        throw new Error(`RuVector request failed: ${response.status} - ${errorBody}`);
      }

      return (await response.json()) as Record<string, unknown>;
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Check if error is a validation error (should not retry)
   */
  private isValidationError(error: Error): boolean {
    return error.message.includes('400') || error.message.includes('validation');
  }

  /**
   * Sleep for specified milliseconds
   */
  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

/**
 * Create an AgentError for RuVector connection failures
 */
export function createRuVectorConnectionError(
  executionRef: string,
  details: Record<string, unknown>
): AgentError {
  return {
    error_code: 'RUVECTOR_CONNECTION_ERROR',
    message: 'Failed to connect to ruvector-service',
    details,
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),
  };
}

/**
 * Create an AgentError for RuVector write failures
 */
export function createRuVectorWriteError(
  executionRef: string,
  details: Record<string, unknown>
): AgentError {
  return {
    error_code: 'RUVECTOR_WRITE_ERROR',
    message: 'Failed to persist DecisionEvent to ruvector-service',
    details,
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),
  };
}
