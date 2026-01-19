/**
 * RuVector Service Client for Memory Retrieval
 *
 * Client for querying and persisting to ruvector-service.
 * This is the ONLY interface for physical persistence.
 *
 * CRITICAL: LLM-Memory-Graph NEVER connects directly to Google SQL.
 * All persistence occurs via ruvector-service client calls only.
 */

import type {
  DecisionEvent,
  AgentError,
  RetrievedNode,
  RetrievedEdge,
  QueryConstraint,
  TraversalOptions,
} from './types.js';

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
 * Response from ruvector-service for persist operations
 */
export interface RuVectorPersistResponse {
  success: boolean;
  event_id?: string;
  error?: string;
  latency_ms?: number;
}

/**
 * Response from ruvector-service for query operations
 */
export interface RuVectorQueryResponse {
  success: boolean;
  nodes: RetrievedNode[];
  edges: RetrievedEdge[];
  total_count?: number;
  error?: string;
  latency_ms?: number;
  cache_hit?: boolean;
}

/**
 * Query parameters for ruvector-service
 */
export interface RuVectorQueryParams {
  anchor_node_ids?: string[];
  anchor_session_ids?: string[];
  constraints?: QueryConstraint[];
  traversal_options?: TraversalOptions;
  semantic_query?: string;
  limit: number;
  offset: number;
}

/**
 * RuVector service client for Memory Retrieval Agent
 */
export class RuVectorClient {
  private readonly config: RuVectorConfig;

  constructor(config: Partial<RuVectorConfig> = {}) {
    this.config = { ...DEFAULT_RUVECTOR_CONFIG, ...config };
  }

  /**
   * Query memory graph via ruvector-service
   *
   * This is the primary method for MEMORY READ operations.
   */
  async queryMemoryGraph(params: RuVectorQueryParams): Promise<RuVectorQueryResponse> {
    const startTime = Date.now();
    let lastError: Error | null = null;

    for (let attempt = 0; attempt < this.config.maxRetries; attempt++) {
      try {
        const response = await this.makeRequest('/api/v1/memory/query', params);
        return {
          success: true,
          nodes: (response.nodes as RetrievedNode[]) ?? [],
          edges: (response.edges as RetrievedEdge[]) ?? [],
          total_count: response.total_count as number | undefined,
          latency_ms: Date.now() - startTime,
          cache_hit: response.cache_hit as boolean | undefined,
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
      nodes: [],
      edges: [],
      error: lastError?.message ?? 'Unknown error',
      latency_ms: Date.now() - startTime,
    };
  }

  /**
   * Execute similarity search via ruvector-service embeddings
   */
  async similaritySearch(
    query: string,
    limit: number = 10,
    filters?: { session_ids?: string[]; node_types?: string[] }
  ): Promise<RuVectorQueryResponse> {
    const startTime = Date.now();

    try {
      const response = await this.makeRequest('/api/v1/memory/similarity', {
        query,
        limit,
        filters,
      });

      return {
        success: true,
        nodes: (response.nodes as RetrievedNode[]) ?? [],
        edges: [],
        total_count: response.total_count as number | undefined,
        latency_ms: Date.now() - startTime,
      };
    } catch (error) {
      return {
        success: false,
        nodes: [],
        edges: [],
        error: error instanceof Error ? error.message : 'Similarity search failed',
        latency_ms: Date.now() - startTime,
      };
    }
  }

  /**
   * Retrieve specific nodes by IDs
   */
  async getNodesByIds(nodeIds: string[]): Promise<RuVectorQueryResponse> {
    const startTime = Date.now();

    try {
      const response = await this.makeRequest('/api/v1/memory/nodes', {
        node_ids: nodeIds,
      });

      return {
        success: true,
        nodes: (response.nodes as RetrievedNode[]) ?? [],
        edges: [],
        latency_ms: Date.now() - startTime,
      };
    } catch (error) {
      return {
        success: false,
        nodes: [],
        edges: [],
        error: error instanceof Error ? error.message : 'Node retrieval failed',
        latency_ms: Date.now() - startTime,
      };
    }
  }

  /**
   * Get edges connected to specified nodes
   */
  async getEdgesForNodes(
    nodeIds: string[],
    direction: 'outgoing' | 'incoming' | 'both' = 'both'
  ): Promise<RuVectorQueryResponse> {
    const startTime = Date.now();

    try {
      const response = await this.makeRequest('/api/v1/memory/edges', {
        node_ids: nodeIds,
        direction,
      });

      return {
        success: true,
        nodes: [],
        edges: (response.edges as RetrievedEdge[]) ?? [],
        latency_ms: Date.now() - startTime,
      };
    } catch (error) {
      return {
        success: false,
        nodes: [],
        edges: [],
        error: error instanceof Error ? error.message : 'Edge retrieval failed',
        latency_ms: Date.now() - startTime,
      };
    }
  }

  /**
   * Persist a DecisionEvent to ruvector-service
   *
   * Every agent invocation MUST emit exactly ONE DecisionEvent.
   */
  async persistDecisionEvent(event: DecisionEvent): Promise<RuVectorPersistResponse> {
    const startTime = Date.now();
    let lastError: Error | null = null;

    for (let attempt = 0; attempt < this.config.maxRetries; attempt++) {
      try {
        const response = await this.makeRequest('/api/v1/decision-events', event);
        return {
          success: true,
          event_id: response.event_id as string | undefined,
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
      return response as unknown as DecisionEvent;
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
          'X-Agent-Id': 'memory-retrieval-agent',
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
 * Create an AgentError for RuVector read failures
 */
export function createRuVectorReadError(
  executionRef: string,
  details: Record<string, unknown>
): AgentError {
  return {
    error_code: 'RUVECTOR_READ_ERROR',
    message: 'Failed to read from ruvector-service',
    details,
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),
  };
}

/**
 * Create an AgentError for query timeout
 */
export function createQueryTimeoutError(
  executionRef: string,
  details: Record<string, unknown>
): AgentError {
  return {
    error_code: 'QUERY_TIMEOUT',
    message: 'Memory query timed out',
    details,
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),
  };
}

/**
 * Create an AgentError for invalid anchor node
 */
export function createInvalidAnchorNodeError(
  executionRef: string,
  nodeIds: string[]
): AgentError {
  return {
    error_code: 'INVALID_ANCHOR_NODE',
    message: 'One or more anchor nodes not found',
    details: { invalid_node_ids: nodeIds },
    execution_ref: executionRef,
    timestamp: new Date().toISOString(),
  };
}
