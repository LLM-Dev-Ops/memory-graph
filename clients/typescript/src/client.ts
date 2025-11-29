/**
 * LLM Memory Graph Client
 *
 * A TypeScript/JavaScript client for the LLM-Memory-Graph gRPC service.
 * Provides a high-level API for interacting with the memory graph with
 * production-ready features including:
 * - Comprehensive error handling
 * - Automatic retry with exponential backoff
 * - Input validation
 * - Connection health monitoring
 * - Graceful shutdown
 *
 * @example
 * ```typescript
 * // Basic usage
 * const client = new MemoryGraphClient({
 *   address: 'localhost:50051',
 *   useTls: false,
 *   retryPolicy: {
 *     maxRetries: 3,
 *     initialBackoff: 100,
 *     maxBackoff: 5000
 *   }
 * });
 *
 * // Create a session with automatic retry
 * const session = await client.createSession({
 *   metadata: { user: 'john' }
 * });
 *
 * // Always close the client when done
 * await client.close();
 * ```
 */

import * as grpc from '@grpc/grpc-js';
import * as protoLoader from '@grpc/proto-loader';
import { promisify } from 'util';
import * as path from 'path';
import {
  ClientConfig,
  Session,
  CreateSessionOptions,
  Node,
  Edge,
  EdgeType,
  EdgeDirection,
  QueryOptions,
  QueryResult,
  AddPromptRequest,
  PromptNode,
  AddResponseRequest,
  ResponseNode,
  AddToolInvocationRequest,
  ToolInvocationNode,
  CreateTemplateRequest,
  TemplateNode,
  InstantiateTemplateRequest,
  StreamOptions,
  EventStreamOptions,
  SessionEventStreamOptions,
  HealthResponse,
  MetricsResponse,
} from './types';
import { mapGrpcError, ConnectionError, TimeoutError } from './errors';
import { withRetry, RetryPolicy, DEFAULT_RETRY_POLICY } from './retry';
import {
  validateSessionId,
  validateNodeId,
  validateAddPromptRequest,
  validateAddResponseRequest,
  validateQueryOptions,
  validateNodeIdArray,
  validateLimit,
  validateOffset,
} from './validators';

const PROTO_PATH = path.join(__dirname, '../../proto/memory_graph.proto');

/**
 * Main client class for LLM Memory Graph
 *
 * Provides a production-ready gRPC client with:
 * - Automatic connection management
 * - Retry logic with exponential backoff
 * - Comprehensive error handling
 * - Input validation
 * - Health monitoring
 */
export class MemoryGraphClient {
  private client: any;
  private config: ClientConfig;
  private connected: boolean = false;
  private retryPolicy: RetryPolicy;
  private healthCheckInterval?: NodeJS.Timeout;
  private reconnectTimeout?: NodeJS.Timeout;
  private closing = false;

  /**
   * Create a new MemoryGraphClient
   *
   * @param config - Client configuration
   * @throws {ValidationError} If configuration is invalid
   * @throws {ConnectionError} If initial connection fails
   *
   * @example
   * ```typescript
   * // Basic configuration
   * const client = new MemoryGraphClient({
   *   address: 'localhost:50051',
   *   useTls: false
   * });
   *
   * // Advanced configuration with retry policy
   * const client = new MemoryGraphClient({
   *   address: 'localhost:50051',
   *   useTls: true,
   *   tlsOptions: {
   *     rootCerts: fs.readFileSync('ca.pem'),
   *     privateKey: fs.readFileSync('key.pem'),
   *     certChain: fs.readFileSync('cert.pem')
   *   },
   *   timeout: 30000,
   *   retryPolicy: {
   *     maxRetries: 5,
   *     initialBackoff: 200,
   *     maxBackoff: 10000,
   *     backoffMultiplier: 2,
   *     useJitter: true,
   *     onRetry: (error, attempt, delay) => {
   *       console.log(`Retry attempt ${attempt} after ${delay}ms`);
   *     }
   *   }
   * });
   * ```
   */
  constructor(config: ClientConfig) {
    this.config = {
      port: 50051,
      useTls: false,
      timeout: 30000,
      ...config,
    };

    // Initialize retry policy
    this.retryPolicy = {
      ...DEFAULT_RETRY_POLICY,
      ...this.config.retryPolicy,
    };

    this.initializeClient();
  }

  /**
   * Initialize the gRPC client
   *
   * @throws {ConnectionError} If client initialization fails
   */
  private initializeClient(): void {
    try {
      const packageDefinition = protoLoader.loadSync(PROTO_PATH, {
        keepCase: true,
        longs: String,
        enums: String,
        defaults: true,
        oneofs: true,
        includeDirs: [path.join(__dirname, '../../proto')],
      });

      const protoDescriptor = grpc.loadPackageDefinition(packageDefinition) as any;
      const memoryGraphProto = protoDescriptor.llm.memory.graph.v1;

      const credentials = this.createCredentials();
      const address = this.getAddress();

      this.client = new memoryGraphProto.MemoryGraphService(address, credentials);
      this.connected = true;
    } catch (error) {
      throw new ConnectionError(
        'Failed to initialize gRPC client',
        { address: this.config.address },
        error as Error
      );
    }
  }

  /**
   * Create gRPC credentials
   */
  private createCredentials(): grpc.ChannelCredentials {
    if (this.config.useTls && this.config.tlsOptions) {
      return grpc.credentials.createSsl(
        this.config.tlsOptions.rootCerts,
        this.config.tlsOptions.privateKey,
        this.config.tlsOptions.certChain
      );
    }
    return grpc.credentials.createInsecure();
  }

  /**
   * Get the server address
   */
  private getAddress(): string {
    if (this.config.port && !this.config.address.includes(':')) {
      return `${this.config.address}:${this.config.port}`;
    }
    return this.config.address;
  }

  /**
   * Convert protobuf timestamp to Date
   */
  private toDate(timestamp: any): Date {
    if (!timestamp) return new Date();
    return new Date(timestamp.seconds * 1000 + timestamp.nanos / 1000000);
  }

  /**
   * Convert Date to protobuf timestamp
   */
  private toTimestamp(date: Date): any {
    const ms = date.getTime();
    return {
      seconds: Math.floor(ms / 1000),
      nanos: (ms % 1000) * 1000000,
    };
  }

  /**
   * Execute a gRPC call with error handling and retry logic
   *
   * @param fn - Function to execute
   * @param retryable - Whether to retry on failure (default: true)
   * @returns Promise with the result
   * @throws {MemoryGraphError} Mapped gRPC error
   *
   * @internal
   */
  private async executeWithRetry<T>(fn: () => Promise<T>, retryable = true): Promise<T> {
    if (!this.connected) {
      throw new ConnectionError('Client is not connected');
    }

    if (this.closing) {
      throw new ConnectionError('Client is closing');
    }

    try {
      if (retryable && this.retryPolicy.maxRetries && this.retryPolicy.maxRetries > 0) {
        return await withRetry(fn, this.retryPolicy);
      } else {
        return await fn();
      }
    } catch (error) {
      throw mapGrpcError(error);
    }
  }

  /**
   * Start periodic health checks
   *
   * @param intervalMs - Interval between health checks in milliseconds
   *
   * @example
   * ```typescript
   * client.startHealthChecks(30000); // Check every 30 seconds
   * ```
   */
  startHealthChecks(intervalMs = 30000): void {
    if (this.healthCheckInterval) {
      return; // Already running
    }

    this.healthCheckInterval = setInterval(async () => {
      try {
        await this.health();
      } catch (error) {
        // Connection lost, attempt to reconnect
        this.connected = false;
        this.attemptReconnect();
      }
    }, intervalMs);
  }

  /**
   * Stop periodic health checks
   */
  stopHealthChecks(): void {
    if (this.healthCheckInterval) {
      clearInterval(this.healthCheckInterval);
      this.healthCheckInterval = undefined;
    }
  }

  /**
   * Attempt to reconnect to the server
   *
   * @internal
   */
  private attemptReconnect(): void {
    if (this.reconnectTimeout || this.closing) {
      return;
    }

    this.reconnectTimeout = setTimeout(() => {
      try {
        this.initializeClient();
        this.reconnectTimeout = undefined;
      } catch (error) {
        this.reconnectTimeout = undefined;
        // Try again after delay
        this.attemptReconnect();
      }
    }, 5000); // Retry after 5 seconds
  }

  /**
   * Wait for the client to be ready
   *
   * @param timeoutMs - Timeout in milliseconds (default: 30000)
   * @returns Promise that resolves when client is ready
   * @throws {TimeoutError} If timeout is reached
   *
   * @example
   * ```typescript
   * await client.waitForReady(5000);
   * console.log('Client is ready!');
   * ```
   */
  async waitForReady(timeoutMs = 30000): Promise<void> {
    const startTime = Date.now();

    while (!this.connected) {
      if (Date.now() - startTime > timeoutMs) {
        throw new TimeoutError(`Client failed to connect within ${timeoutMs}ms`, {
          timeout: timeoutMs,
        });
      }
      await new Promise((resolve) => setTimeout(resolve, 100));
    }

    // Verify connection with health check
    try {
      await this.health();
    } catch (error) {
      throw new ConnectionError('Health check failed', undefined, error as Error);
    }
  }

  // ============================================================================
  // Session Management
  // ============================================================================

  /**
   * Create a new session
   *
   * @param options - Session creation options
   * @returns Promise with the created session
   * @throws {ValidationError} If options are invalid
   * @throws {MemoryGraphError} If creation fails
   *
   * @example
   * ```typescript
   * const session = await client.createSession({
   *   metadata: { user: 'john', context: 'chat' }
   * });
   * console.log('Session ID:', session.id);
   * ```
   */
  async createSession(options: CreateSessionOptions = {}): Promise<Session> {
    return this.executeWithRetry(async () => {
      const createSession = promisify(this.client.createSession.bind(this.client));
      const response = await createSession({
        metadata: options.metadata || {},
      });

      return {
        id: response.id,
        createdAt: this.toDate(response.created_at),
        updatedAt: this.toDate(response.updated_at),
        metadata: response.metadata || {},
        isActive: response.is_active,
      };
    });
  }

  /**
   * Get a session by ID
   *
   * @param sessionId - The session ID
   * @returns Promise with the session
   * @throws {ValidationError} If sessionId is invalid
   * @throws {NotFoundError} If session is not found
   * @throws {MemoryGraphError} If retrieval fails
   */
  async getSession(sessionId: string): Promise<Session> {
    validateSessionId(sessionId);

    return this.executeWithRetry(async () => {
      const getSession = promisify(this.client.getSession.bind(this.client));
      const response = await getSession({ session_id: sessionId });

      return {
        id: response.id,
        createdAt: this.toDate(response.created_at),
        updatedAt: this.toDate(response.updated_at),
        metadata: response.metadata || {},
        isActive: response.is_active,
      };
    });
  }

  /**
   * Delete a session
   *
   * @param sessionId - The session ID to delete
   * @throws {ValidationError} If sessionId is invalid
   * @throws {NotFoundError} If session is not found
   * @throws {MemoryGraphError} If deletion fails
   */
  async deleteSession(sessionId: string): Promise<void> {
    validateSessionId(sessionId);

    return this.executeWithRetry(async () => {
      const deleteSession = promisify(this.client.deleteSession.bind(this.client));
      await deleteSession({ session_id: sessionId });
    }, false); // Don't retry deletes
  }

  /**
   * List sessions
   *
   * @param limit - Maximum number of sessions to return (default: 100, max: 10000)
   * @param offset - Number of sessions to skip (default: 0)
   * @returns Promise with sessions and total count
   * @throws {ValidationError} If parameters are invalid
   * @throws {MemoryGraphError} If listing fails
   */
  async listSessions(
    limit: number = 100,
    offset: number = 0
  ): Promise<{ sessions: Session[]; totalCount: number }> {
    validateLimit(limit);
    validateOffset(offset);

    return this.executeWithRetry(async () => {
      const listSessions = promisify(this.client.listSessions.bind(this.client));
      const response = await listSessions({ limit, offset });

      return {
        sessions: (response.sessions || []).map((s: any) => ({
          id: s.id,
          createdAt: this.toDate(s.created_at),
          updatedAt: this.toDate(s.updated_at),
          metadata: s.metadata || {},
          isActive: s.is_active,
        })),
        totalCount: parseInt(response.total_count || '0'),
      };
    });
  }

  // ============================================================================
  // Node Operations
  // ============================================================================

  /**
   * Create a node
   *
   * @param node - The node to create
   * @returns Promise with the created node
   */
  async createNode(node: Node): Promise<Node> {
    const createNode = promisify(this.client.createNode.bind(this.client));
    const response = await createNode({ node });
    return this.parseNode(response);
  }

  /**
   * Get a node by ID
   *
   * @param nodeId - The node ID
   * @returns Promise with the node
   * @throws {ValidationError} If nodeId is invalid
   * @throws {NotFoundError} If node is not found
   * @throws {MemoryGraphError} If retrieval fails
   */
  async getNode(nodeId: string): Promise<Node> {
    validateNodeId(nodeId);

    return this.executeWithRetry(async () => {
      const getNode = promisify(this.client.getNode.bind(this.client));
      const response = await getNode({ node_id: nodeId });
      return this.parseNode(response);
    });
  }

  /**
   * Update a node
   *
   * @param node - The node to update
   * @returns Promise with the updated node
   */
  async updateNode(node: Node): Promise<Node> {
    const updateNode = promisify(this.client.updateNode.bind(this.client));
    const response = await updateNode({ node });
    return this.parseNode(response);
  }

  /**
   * Delete a node
   *
   * @param nodeId - The node ID to delete
   */
  async deleteNode(nodeId: string): Promise<void> {
    const deleteNode = promisify(this.client.deleteNode.bind(this.client));
    await deleteNode({ node_id: nodeId });
  }

  /**
   * Batch create nodes
   *
   * @param nodes - Array of nodes to create
   * @returns Promise with created nodes and count
   */
  async batchCreateNodes(nodes: Node[]): Promise<{ nodes: Node[]; createdCount: number }> {
    const batchCreateNodes = promisify(this.client.batchCreateNodes.bind(this.client));
    const response = await batchCreateNodes({ nodes });

    return {
      nodes: (response.nodes || []).map((n: any) => this.parseNode(n)),
      createdCount: response.created_count,
    };
  }

  /**
   * Batch get nodes
   *
   * @param nodeIds - Array of node IDs
   * @returns Promise with array of nodes
   * @throws {ValidationError} If nodeIds array is invalid
   * @throws {MemoryGraphError} If retrieval fails
   */
  async batchGetNodes(nodeIds: string[]): Promise<Node[]> {
    validateNodeIdArray(nodeIds);

    return this.executeWithRetry(async () => {
      const batchGetNodes = promisify(this.client.batchGetNodes.bind(this.client));
      const response = await batchGetNodes({ node_ids: nodeIds });
      return (response.nodes || []).map((n: any) => this.parseNode(n));
    });
  }

  /**
   * Parse a node from the protobuf response
   */
  private parseNode(response: any): Node {
    return {
      id: response.id,
      type: response.type,
      createdAt: this.toDate(response.created_at),
      data:
        response.prompt ||
        response.response ||
        response.tool_invocation ||
        response.agent ||
        response.template,
    };
  }

  // ============================================================================
  // Edge Operations
  // ============================================================================

  /**
   * Create an edge
   *
   * @param edge - The edge to create
   * @returns Promise with the created edge
   */
  async createEdge(edge: Edge): Promise<Edge> {
    const createEdge = promisify(this.client.createEdge.bind(this.client));
    const response = await createEdge({ edge });
    return this.parseEdge(response);
  }

  /**
   * Get edges for a node
   *
   * @param nodeId - The node ID
   * @param direction - Edge direction (optional)
   * @param type - Edge type (optional)
   * @returns Promise with array of edges
   */
  async getEdges(nodeId: string, direction?: EdgeDirection, type?: EdgeType): Promise<Edge[]> {
    const getEdges = promisify(this.client.getEdges.bind(this.client));
    const response = await getEdges({
      node_id: nodeId,
      direction,
      type,
    });

    return (response.edges || []).map((e: any) => this.parseEdge(e));
  }

  /**
   * Delete an edge
   *
   * @param edgeId - The edge ID to delete
   */
  async deleteEdge(edgeId: string): Promise<void> {
    const deleteEdge = promisify(this.client.deleteEdge.bind(this.client));
    await deleteEdge({ edge_id: edgeId });
  }

  /**
   * Parse an edge from the protobuf response
   */
  private parseEdge(response: any): Edge {
    return {
      id: response.id,
      fromNodeId: response.from_node_id,
      toNodeId: response.to_node_id,
      type: response.type,
      createdAt: this.toDate(response.created_at),
      properties: response.properties || {},
    };
  }

  // ============================================================================
  // Query Operations
  // ============================================================================

  /**
   * Query nodes
   *
   * @param options - Query options
   * @returns Promise with query results
   * @throws {ValidationError} If options are invalid
   * @throws {MemoryGraphError} If query fails
   *
   * @example
   * ```typescript
   * const results = await client.query({
   *   sessionId: 'session-123',
   *   nodeType: NodeType.PROMPT,
   *   limit: 10
   * });
   * console.log('Found', results.totalCount, 'nodes');
   * ```
   */
  async query(options: QueryOptions = {}): Promise<QueryResult> {
    validateQueryOptions(options);

    return this.executeWithRetry(async () => {
      const query = promisify(this.client.query.bind(this.client));
      const request: any = {
        limit: options.limit || 100,
        offset: options.offset || 0,
      };

      if (options.sessionId) request.session_id = options.sessionId;
      if (options.nodeType !== undefined) request.node_type = options.nodeType;
      if (options.after) request.after = this.toTimestamp(options.after);
      if (options.before) request.before = this.toTimestamp(options.before);
      if (options.filters) request.filters = options.filters;

      const response = await query(request);

      return {
        nodes: (response.nodes || []).map((n: any) => this.parseNode(n)),
        totalCount: parseInt(response.total_count || '0'),
      };
    });
  }

  /**
   * Stream query results
   *
   * @param options - Query options
   * @param streamOptions - Stream callbacks
   * @example
   * ```typescript
   * client.streamQuery(
   *   { sessionId: 'session-123' },
   *   {
   *     onData: (node) => console.log('Received node:', node.id),
   *     onError: (error) => console.error('Stream error:', error),
   *     onEnd: () => console.log('Stream ended')
   *   }
   * );
   * ```
   */
  streamQuery(options: QueryOptions, streamOptions: StreamOptions): void {
    const request: any = {};
    if (options.sessionId) request.session_id = options.sessionId;
    if (options.nodeType !== undefined) request.node_type = options.nodeType;
    if (options.after) request.after = this.toTimestamp(options.after);
    if (options.before) request.before = this.toTimestamp(options.before);
    if (options.limit) request.limit = options.limit;
    if (options.offset) request.offset = options.offset;
    if (options.filters) request.filters = options.filters;

    const stream = this.client.streamQuery(request);

    stream.on('data', (node: any) => {
      streamOptions.onData(this.parseNode(node));
    });

    if (streamOptions.onError) {
      stream.on('error', streamOptions.onError);
    }

    if (streamOptions.onEnd) {
      stream.on('end', streamOptions.onEnd);
    }
  }

  // ============================================================================
  // Prompt & Response Operations
  // ============================================================================

  /**
   * Add a prompt to a session
   *
   * @param request - Add prompt request
   * @returns Promise with the created prompt node
   * @throws {ValidationError} If request is invalid
   * @throws {NotFoundError} If session is not found
   * @throws {MemoryGraphError} If creation fails
   *
   * @example
   * ```typescript
   * const prompt = await client.addPrompt({
   *   sessionId: 'session-123',
   *   content: 'What is the capital of France?',
   *   metadata: {
   *     model: 'gpt-4',
   *     temperature: 0.7,
   *     toolsAvailable: ['search', 'calculator'],
   *     custom: {}
   *   }
   * });
   * ```
   */
  async addPrompt(request: AddPromptRequest): Promise<PromptNode> {
    validateAddPromptRequest(request);

    return this.executeWithRetry(async () => {
      const addPrompt = promisify(this.client.addPrompt.bind(this.client));
      const response = await addPrompt(request);
      return response as PromptNode;
    });
  }

  /**
   * Add a response to a prompt
   *
   * @param request - Add response request
   * @returns Promise with the created response node
   * @throws {ValidationError} If request is invalid
   * @throws {NotFoundError} If prompt is not found
   * @throws {MemoryGraphError} If creation fails
   *
   * @example
   * ```typescript
   * const response = await client.addResponse({
   *   promptId: prompt.id,
   *   content: 'The capital of France is Paris.',
   *   tokenUsage: {
   *     promptTokens: 15,
   *     completionTokens: 8,
   *     totalTokens: 23
   *   },
   *   metadata: {
   *     model: 'gpt-4',
   *     finishReason: 'stop',
   *     latencyMs: 1234,
   *     custom: {}
   *   }
   * });
   * ```
   */
  async addResponse(request: AddResponseRequest): Promise<ResponseNode> {
    validateAddResponseRequest(request);

    return this.executeWithRetry(async () => {
      const addResponse = promisify(this.client.addResponse.bind(this.client));
      const response = await addResponse(request);
      return response as ResponseNode;
    });
  }

  /**
   * Add a tool invocation
   *
   * @param request - Add tool invocation request
   * @returns Promise with the created tool invocation node
   */
  async addToolInvocation(request: AddToolInvocationRequest): Promise<ToolInvocationNode> {
    const addToolInvocation = promisify(this.client.addToolInvocation.bind(this.client));
    const response = await addToolInvocation(request);
    return response as ToolInvocationNode;
  }

  // ============================================================================
  // Template Operations
  // ============================================================================

  /**
   * Create a template
   *
   * @param request - Create template request
   * @returns Promise with the created template node
   */
  async createTemplate(request: CreateTemplateRequest): Promise<TemplateNode> {
    const createTemplate = promisify(this.client.createTemplate.bind(this.client));
    const response = await createTemplate(request);
    return response as TemplateNode;
  }

  /**
   * Instantiate a template
   *
   * @param request - Instantiate template request
   * @returns Promise with the created prompt node
   */
  async instantiateTemplate(request: InstantiateTemplateRequest): Promise<PromptNode> {
    const instantiateTemplate = promisify(this.client.instantiateTemplate.bind(this.client));
    const response = await instantiateTemplate(request);
    return response as PromptNode;
  }

  // ============================================================================
  // Streaming Operations
  // ============================================================================

  /**
   * Stream events
   *
   * @param options - Event stream options
   */
  streamEvents(options: EventStreamOptions): void {
    const request: any = {};
    if (options.sessionId) request.session_id = options.sessionId;
    if (options.eventTypes) request.event_types = options.eventTypes;

    const stream = this.client.streamEvents(request);

    stream.on('data', (event: any) => {
      options.onData({
        id: event.id,
        type: event.type,
        timestamp: this.toDate(event.timestamp),
        payload: event.payload,
      });
    });

    if (options.onError) {
      stream.on('error', options.onError);
    }

    if (options.onEnd) {
      stream.on('end', options.onEnd);
    }
  }

  /**
   * Subscribe to session events
   *
   * @param options - Session event stream options
   */
  subscribeToSession(options: SessionEventStreamOptions): void {
    const stream = this.client.subscribeToSession({ session_id: options.sessionId });

    stream.on('data', (sessionEvent: any) => {
      options.onData({
        event: {
          id: sessionEvent.event.id,
          type: sessionEvent.event.type,
          timestamp: this.toDate(sessionEvent.event.timestamp),
          payload: sessionEvent.event.payload,
        },
        sessionId: sessionEvent.session_id,
      });
    });

    if (options.onError) {
      stream.on('error', options.onError);
    }

    if (options.onEnd) {
      stream.on('end', options.onEnd);
    }
  }

  // ============================================================================
  // Health & Metrics
  // ============================================================================

  /**
   * Check service health
   *
   * @returns Promise with health response
   */
  async health(): Promise<HealthResponse> {
    const health = promisify(this.client.health.bind(this.client));
    const response = await health({});
    return {
      status: response.status,
      version: response.version,
      uptimeSeconds: parseInt(response.uptime_seconds || '0'),
    };
  }

  /**
   * Get service metrics
   *
   * @returns Promise with metrics response
   */
  async getMetrics(): Promise<MetricsResponse> {
    const getMetrics = promisify(this.client.getMetrics.bind(this.client));
    const response = await getMetrics({});
    return {
      totalNodes: parseInt(response.total_nodes || '0'),
      totalEdges: parseInt(response.total_edges || '0'),
      totalSessions: parseInt(response.total_sessions || '0'),
      activeSessions: parseInt(response.active_sessions || '0'),
      avgWriteLatencyMs: parseFloat(response.avg_write_latency_ms || '0'),
      avgReadLatencyMs: parseFloat(response.avg_read_latency_ms || '0'),
      requestsPerSecond: parseInt(response.requests_per_second || '0'),
    };
  }

  /**
   * Close the client connection gracefully
   *
   * Stops health checks, clears timeouts, and closes the gRPC connection.
   * This is the recommended way to shut down the client.
   *
   * @param timeoutMs - Maximum time to wait for graceful shutdown (default: 5000ms)
   * @returns Promise that resolves when client is closed
   *
   * @example
   * ```typescript
   * await client.close();
   * console.log('Client closed successfully');
   * ```
   */
  async close(timeoutMs = 5000): Promise<void> {
    if (!this.connected && !this.closing) {
      return; // Already closed
    }

    this.closing = true;

    // Stop health checks
    this.stopHealthChecks();

    // Clear reconnect timeout
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = undefined;
    }

    // Close the gRPC client
    if (this.client) {
      // Wait for client to close gracefully or timeout
      const closePromise = new Promise<void>((resolve) => {
        this.client.close();
        resolve();
      });

      const timeoutPromise = new Promise<void>((resolve) => {
        setTimeout(resolve, timeoutMs);
      });

      await Promise.race([closePromise, timeoutPromise]);

      this.connected = false;
      this.client = null;
    }

    this.closing = false;
  }

  /**
   * Close the client connection synchronously (deprecated)
   *
   * @deprecated Use async close() method instead
   */
  closeSync(): void {
    this.stopHealthChecks();

    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = undefined;
    }

    if (this.client) {
      this.client.close();
      this.connected = false;
      this.client = null;
    }
  }

  /**
   * Check if client is connected
   *
   * @returns True if client is connected and ready
   */
  isConnected(): boolean {
    return this.connected && !this.closing;
  }

  /**
   * Check if client is closing
   *
   * @returns True if client is in the process of closing
   */
  isClosing(): boolean {
    return this.closing;
  }

  /**
   * Get client configuration
   *
   * @returns Current client configuration (read-only copy)
   */
  getConfig(): Readonly<ClientConfig> {
    return { ...this.config };
  }

  /**
   * Get retry policy
   *
   * @returns Current retry policy (read-only copy)
   */
  getRetryPolicy(): Readonly<RetryPolicy> {
    return { ...this.retryPolicy };
  }

  /**
   * Update retry policy
   *
   * @param policy - New retry policy settings to merge
   *
   * @example
   * ```typescript
   * client.updateRetryPolicy({
   *   maxRetries: 5,
   *   initialBackoff: 200
   * });
   * ```
   */
  updateRetryPolicy(policy: Partial<RetryPolicy>): void {
    this.retryPolicy = {
      ...this.retryPolicy,
      ...policy,
    };
  }
}
