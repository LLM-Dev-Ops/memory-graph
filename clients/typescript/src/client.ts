/**
 * LLM Memory Graph Client
 *
 * A TypeScript/JavaScript client for the LLM-Memory-Graph gRPC service.
 * Provides a high-level API for interacting with the memory graph.
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

const PROTO_PATH = path.join(__dirname, '../../proto/memory_graph.proto');

/**
 * Main client class for LLM Memory Graph
 */
export class MemoryGraphClient {
  private client: any;
  private config: ClientConfig;
  private connected: boolean = false;

  /**
   * Create a new MemoryGraphClient
   *
   * @param config - Client configuration
   * @example
   * ```typescript
   * const client = new MemoryGraphClient({
   *   address: 'localhost:50051',
   *   useTls: false
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
    this.initializeClient();
  }

  /**
   * Initialize the gRPC client
   */
  private initializeClient(): void {
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

  // ============================================================================
  // Session Management
  // ============================================================================

  /**
   * Create a new session
   *
   * @param options - Session creation options
   * @returns Promise with the created session
   * @example
   * ```typescript
   * const session = await client.createSession({
   *   metadata: { user: 'john', context: 'chat' }
   * });
   * console.log('Session ID:', session.id);
   * ```
   */
  async createSession(options: CreateSessionOptions = {}): Promise<Session> {
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
  }

  /**
   * Get a session by ID
   *
   * @param sessionId - The session ID
   * @returns Promise with the session
   */
  async getSession(sessionId: string): Promise<Session> {
    const getSession = promisify(this.client.getSession.bind(this.client));
    const response = await getSession({ session_id: sessionId });

    return {
      id: response.id,
      createdAt: this.toDate(response.created_at),
      updatedAt: this.toDate(response.updated_at),
      metadata: response.metadata || {},
      isActive: response.is_active,
    };
  }

  /**
   * Delete a session
   *
   * @param sessionId - The session ID to delete
   */
  async deleteSession(sessionId: string): Promise<void> {
    const deleteSession = promisify(this.client.deleteSession.bind(this.client));
    await deleteSession({ session_id: sessionId });
  }

  /**
   * List sessions
   *
   * @param limit - Maximum number of sessions to return
   * @param offset - Number of sessions to skip
   * @returns Promise with sessions and total count
   */
  async listSessions(
    limit: number = 100,
    offset: number = 0
  ): Promise<{ sessions: Session[]; totalCount: number }> {
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
   */
  async getNode(nodeId: string): Promise<Node> {
    const getNode = promisify(this.client.getNode.bind(this.client));
    const response = await getNode({ node_id: nodeId });
    return this.parseNode(response);
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
   */
  async batchGetNodes(nodeIds: string[]): Promise<Node[]> {
    const batchGetNodes = promisify(this.client.batchGetNodes.bind(this.client));
    const response = await batchGetNodes({ node_ids: nodeIds });
    return (response.nodes || []).map((n: any) => this.parseNode(n));
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
    const addPrompt = promisify(this.client.addPrompt.bind(this.client));
    const response = await addPrompt(request);
    return response as PromptNode;
  }

  /**
   * Add a response to a prompt
   *
   * @param request - Add response request
   * @returns Promise with the created response node
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
    const addResponse = promisify(this.client.addResponse.bind(this.client));
    const response = await addResponse(request);
    return response as ResponseNode;
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
   * Close the client connection
   */
  close(): void {
    if (this.client) {
      this.client.close();
      this.connected = false;
    }
  }

  /**
   * Check if client is connected
   */
  isConnected(): boolean {
    return this.connected;
  }
}
