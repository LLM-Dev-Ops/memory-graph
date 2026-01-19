/**
 * RuVector Service Client for Long-Term Pattern Agent
 *
 * Client for reading and persisting data to ruvector-service.
 * This is the ONLY interface for physical persistence.
 *
 * CRITICAL: LLM-Memory-Graph NEVER connects directly to Google SQL.
 * All persistence occurs via ruvector-service client calls only.
 *
 * This agent is MEMORY ANALYSIS classification, so it:
 * - READS historical memory data for pattern analysis
 * - WRITES DecisionEvents recording the analysis results
 */

import type { DecisionEvent, AgentError } from './types.js';

/**
 * RuVector service configuration
 */
export interface RuVectorConfig {
  baseUrl: string;
  apiKey: string;
  timeoutMs: number;
  maxRetries: number;
  retryDelayMs: number;
}

/**
 * Default configuration
 */
export const DEFAULT_RUVECTOR_CONFIG: RuVectorConfig = {
  baseUrl: process.env['RUVECTOR_SERVICE_URL'] ?? 'http://localhost:8080',
  apiKey: process.env['RUVECTOR_API_KEY'] ?? '',
  timeoutMs: 60000, // Longer timeout for analysis queries
  maxRetries: 3,
  retryDelayMs: 1000,
};

/**
 * Response from ruvector-service for write operations
 */
export interface RuVectorWriteResponse {
  success: boolean;
  event_id?: string;
  error?: string;
  latency_ms?: number;
}

/**
 * Response from ruvector-service for read operations
 */
export interface RuVectorReadResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  latency_ms?: number;
}

/**
 * Memory node structure from ruvector-service
 */
export interface MemoryNode {
  node_id: string;
  node_type: string;
  session_id: string;
  content_hash?: string;
  timestamp: string;
  properties: Record<string, unknown>;
}

/**
 * Memory edge structure from ruvector-service
 */
export interface MemoryEdge {
  edge_id: string;
  edge_type: string;
  from_node_id: string;
  to_node_id: string;
  properties?: Record<string, unknown>;
}

/**
 * Query parameters for fetching memory data
 */
export interface MemoryQueryParams {
  session_ids?: string[];
  agent_ids?: string[];
  user_ids?: string[];
  tags?: string[];
  from_timestamp: string;
  to_timestamp: string;
  node_types?: string[];
  limit?: number;
  offset?: number;
}

/**
 * Memory data result from query
 */
export interface MemoryQueryResult {
  nodes: MemoryNode[];
  edges: MemoryEdge[];
  total_nodes: number;
  total_edges: number;
}

/**
 * RuVector service client for Long-Term Pattern Agent
 */
export class RuVectorClient {
  private readonly config: RuVectorConfig;

  constructor(config: Partial<RuVectorConfig> = {}) {
    this.config = { ...DEFAULT_RUVECTOR_CONFIG, ...config };
  }

  /**
   * Query memory nodes and edges for pattern analysis
   *
   * This is the primary READ method for the MEMORY ANALYSIS agent.
   */
  async queryMemory(params: MemoryQueryParams): Promise<RuVectorReadResponse<MemoryQueryResult>> {
    const startTime = Date.now();

    try {
      const queryParams = new URLSearchParams();
      queryParams.set('from_timestamp', params.from_timestamp);
      queryParams.set('to_timestamp', params.to_timestamp);

      if (params.session_ids?.length) {
        queryParams.set('session_ids', params.session_ids.join(','));
      }
      if (params.agent_ids?.length) {
        queryParams.set('agent_ids', params.agent_ids.join(','));
      }
      if (params.user_ids?.length) {
        queryParams.set('user_ids', params.user_ids.join(','));
      }
      if (params.tags?.length) {
        queryParams.set('tags', params.tags.join(','));
      }
      if (params.node_types?.length) {
        queryParams.set('node_types', params.node_types.join(','));
      }
      if (params.limit !== undefined) {
        queryParams.set('limit', String(params.limit));
      }
      if (params.offset !== undefined) {
        queryParams.set('offset', String(params.offset));
      }

      const response = await this.makeRequest<MemoryQueryResult>(
        `/api/v1/memory/query?${queryParams.toString()}`,
        undefined,
        'GET'
      );

      return {
        success: true,
        data: response,
        latency_ms: Date.now() - startTime,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
        latency_ms: Date.now() - startTime,
      };
    }
  }

  /**
   * Query DecisionEvents for historical pattern data
   */
  async queryDecisionEvents(query: {
    agent_id?: string;
    decision_type?: string;
    from_timestamp?: string;
    to_timestamp?: string;
    limit?: number;
  }): Promise<RuVectorReadResponse<DecisionEvent[]>> {
    const startTime = Date.now();

    try {
      const params = new URLSearchParams();
      for (const [key, value] of Object.entries(query)) {
        if (value !== undefined) {
          params.set(key, String(value));
        }
      }

      const response = await this.makeRequest<{ events: DecisionEvent[] }>(
        `/api/v1/decision-events?${params.toString()}`,
        undefined,
        'GET'
      );

      return {
        success: true,
        data: response.events ?? [],
        latency_ms: Date.now() - startTime,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
        latency_ms: Date.now() - startTime,
      };
    }
  }

  /**
   * Persist a DecisionEvent to ruvector-service
   *
   * Every agent invocation MUST emit exactly ONE DecisionEvent.
   */
  async persistDecisionEvent(event: DecisionEvent): Promise<RuVectorWriteResponse> {
    const startTime = Date.now();
    let lastError: Error | null = null;

    for (let attempt = 0; attempt < this.config.maxRetries; attempt++) {
      try {
        const response = await this.makeRequest<{ event_id: string }>(
          '/api/v1/decision-events',
          event
        );
        return {
          success: true,
          event_id: response.event_id,
          latency_ms: Date.now() - startTime,
        };
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));

        if (this.isValidationError(lastError)) {
          break;
        }

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
      const response = await this.makeRequest<DecisionEvent>(
        `/api/v1/decision-events/${executionRef}`,
        undefined,
        'GET'
      );
      return response;
    } catch {
      return null;
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
  private async makeRequest<T>(
    path: string,
    body?: unknown,
    method: 'GET' | 'POST' = 'POST'
  ): Promise<T> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeoutMs);

    try {
      const response = await fetch(`${this.config.baseUrl}${path}`, {
        method,
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${this.config.apiKey}`,
          'X-Agent-Id': 'long-term-pattern-agent',
        },
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });

      if (!response.ok) {
        const errorBody = await response.text();
        throw new Error(`RuVector request failed: ${response.status} - ${errorBody}`);
      }

      return (await response.json()) as T;
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
 * Create an AgentError for RuVector read failures
 */
export function createRuVectorReadError(
  executionRef: string,
  details: Record<string, unknown>
): AgentError {
  return {
    error_code: 'RUVECTOR_READ_ERROR',
    message: 'Failed to read data from ruvector-service',
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

/**
 * Create an AgentError for insufficient data
 */
export function createInsufficientDataError(
  executionRef: string,
  details: Record<string, unknown>
): AgentError {
  return {
    error_code: 'INSUFFICIENT_DATA',
    message: 'Insufficient data available for pattern analysis',
    details,
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),
  };
}

/**
 * Create an AgentError for analysis timeout
 */
export function createAnalysisTimeoutError(
  executionRef: string,
  details: Record<string, unknown>
): AgentError {
  return {
    error_code: 'ANALYSIS_TIMEOUT',
    message: 'Pattern analysis exceeded time limit',
    details,
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),
  };
}
