# LLM-Memory-Graph Integration Implementation Guide

## Overview

This document provides practical implementation guidance, code examples, and reference implementations for the integration patterns defined in INTEGRATION_ARCHITECTURE.md.

---

## 1. OBSERVATORY INTEGRATION - CODE EXAMPLES

### 1.1 Kafka Consumer Implementation (TypeScript/Node.js)

```typescript
import { Kafka, Consumer, EachMessagePayload } from 'kafkajs';
import { CircuitBreaker } from 'opossum';
import { Counter, Histogram, Gauge } from 'prom-client';

// Metrics
const eventsReceived = new Counter({
  name: 'llm_memory_graph_events_received_total',
  help: 'Total events received from Observatory',
  labelNames: ['event_type', 'source']
});

const processingDuration = new Histogram({
  name: 'llm_memory_graph_event_processing_duration_seconds',
  help: 'Event processing duration',
  labelNames: ['event_type'],
  buckets: [0.01, 0.05, 0.1, 0.5, 1, 2, 5]
});

const bufferUtilization = new Gauge({
  name: 'llm_memory_graph_buffer_utilization_percent',
  help: 'Buffer utilization percentage',
  labelNames: ['consumer_id']
});

interface ObservatoryConfig {
  brokers: string[];
  topic: string;
  consumerGroup: string;
  maxBatchSize: number;
  bufferSizeMB: number;
}

class ObservatoryEventConsumer {
  private kafka: Kafka;
  private consumer: Consumer;
  private config: ObservatoryConfig;
  private circuitBreaker: CircuitBreaker;
  private eventBuffer: RingBuffer<Event>;

  constructor(config: ObservatoryConfig) {
    this.config = config;

    this.kafka = new Kafka({
      clientId: 'llm-memory-graph',
      brokers: config.brokers,
      ssl: true,
      sasl: {
        mechanism: 'scram-sha-512',
        username: process.env.KAFKA_USERNAME!,
        password: process.env.KAFKA_PASSWORD!
      }
    });

    this.consumer = this.kafka.consumer({
      groupId: config.consumerGroup,
      sessionTimeout: 30000,
      heartbeatInterval: 3000,
      maxBytesPerPartition: 1048576 // 1MB
    });

    // Ring buffer for backpressure handling
    const bufferSize = (config.bufferSizeMB * 1024 * 1024) / 1024; // Estimate 1KB per event
    this.eventBuffer = new RingBuffer<Event>(bufferSize);

    // Circuit breaker for graph writes
    this.circuitBreaker = new CircuitBreaker(this.writeToGraph.bind(this), {
      timeout: 5000,
      errorThresholdPercentage: 50,
      resetTimeout: 30000
    });

    this.circuitBreaker.on('open', () => {
      console.error('Circuit breaker OPEN - graph write operations halted');
    });

    this.circuitBreaker.on('halfOpen', () => {
      console.warn('Circuit breaker HALF_OPEN - testing graph availability');
    });
  }

  async start(): Promise<void> {
    await this.consumer.connect();
    await this.consumer.subscribe({
      topic: this.config.topic,
      fromBeginning: false
    });

    await this.consumer.run({
      eachMessage: this.handleMessage.bind(this),
      eachBatch: this.handleBatch.bind(this)
    });

    // Start buffer processor
    this.startBufferProcessor();
  }

  private async handleMessage(payload: EachMessagePayload): Promise<void> {
    const { topic, partition, message } = payload;
    const startTime = Date.now();

    try {
      const event = this.parseEvent(message.value);

      eventsReceived.inc({
        event_type: event.event_type,
        source: 'observatory'
      });

      // Add to buffer with backpressure handling
      const added = this.eventBuffer.push(event);
      if (!added) {
        console.warn(`Buffer full, dropping event: ${event.event_id}`);
        this.eventsDropped.inc({
          event_type: event.event_type,
          reason: 'buffer_full'
        });
      }

      // Update buffer utilization metric
      bufferUtilization.set(
        { consumer_id: this.config.consumerGroup },
        this.eventBuffer.utilization()
      );

      const duration = (Date.now() - startTime) / 1000;
      processingDuration.observe({ event_type: event.event_type }, duration);

    } catch (error) {
      console.error('Error processing message:', error);
      throw error; // Let Kafka handle retry
    }
  }

  private async handleBatch(payload: EachBatchPayload): Promise<void> {
    const { batch } = payload;
    const events: Event[] = [];

    for (const message of batch.messages) {
      try {
        const event = this.parseEvent(message.value);
        events.push(event);
      } catch (error) {
        console.error('Error parsing event:', error);
      }
    }

    // Batch write to graph
    if (events.length > 0) {
      await this.batchWriteToGraph(events);
    }
  }

  private parseEvent(messageValue: Buffer | null): Event {
    if (!messageValue) throw new Error('Empty message');

    const parsed = JSON.parse(messageValue.toString());

    return {
      event_id: parsed.event_id,
      event_type: parsed.event_type,
      timestamp: parsed.timestamp,
      payload: parsed.payload,
      trace_id: parsed.trace_id,
      span_id: parsed.span_id,
      metadata: parsed.metadata
    };
  }

  private startBufferProcessor(): void {
    setInterval(async () => {
      const batch = this.eventBuffer.drain(this.config.maxBatchSize);
      if (batch.length > 0) {
        await this.batchWriteToGraph(batch);
      }
    }, 100); // Process every 100ms
  }

  private async writeToGraph(event: Event): Promise<void> {
    const graphWriter = new GraphWriter();
    await graphWriter.createInvocationNode(event);
  }

  private async batchWriteToGraph(events: Event[]): Promise<void> {
    try {
      await this.circuitBreaker.fire(events);
    } catch (error) {
      console.error('Batch write failed:', error);
      // Send to dead letter queue
      await this.sendToDeadLetterQueue(events);
    }
  }

  private async sendToDeadLetterQueue(events: Event[]): Promise<void> {
    const dlqProducer = this.kafka.producer();
    await dlqProducer.connect();

    const messages = events.map(event => ({
      key: event.event_id,
      value: JSON.stringify(event),
      headers: {
        'x-original-topic': this.config.topic,
        'x-failure-reason': 'graph_write_failure',
        'x-retry-count': '0'
      }
    }));

    await dlqProducer.send({
      topic: `${this.config.topic}-dlq`,
      messages
    });

    await dlqProducer.disconnect();
  }

  async shutdown(): Promise<void> {
    await this.consumer.disconnect();
  }
}

// Ring Buffer implementation
class RingBuffer<T> {
  private buffer: (T | null)[];
  private head: number = 0;
  private tail: number = 0;
  private size: number = 0;
  private capacity: number;

  constructor(capacity: number) {
    this.capacity = capacity;
    this.buffer = new Array(capacity).fill(null);
  }

  push(item: T): boolean {
    if (this.size === this.capacity) {
      // Buffer full, drop oldest
      this.head = (this.head + 1) % this.capacity;
      this.size--;
    }

    this.buffer[this.tail] = item;
    this.tail = (this.tail + 1) % this.capacity;
    this.size++;
    return true;
  }

  drain(count: number): T[] {
    const result: T[] = [];
    const toDrain = Math.min(count, this.size);

    for (let i = 0; i < toDrain; i++) {
      const item = this.buffer[this.head];
      if (item !== null) {
        result.push(item);
      }
      this.head = (this.head + 1) % this.capacity;
      this.size--;
    }

    return result;
  }

  utilization(): number {
    return (this.size / this.capacity) * 100;
  }
}

// Graph Writer with Neo4j
import neo4j, { Driver, Session } from 'neo4j-driver';

class GraphWriter {
  private driver: Driver;

  constructor() {
    this.driver = neo4j.driver(
      process.env.NEO4J_URI!,
      neo4j.auth.basic(
        process.env.NEO4J_USERNAME!,
        process.env.NEO4J_PASSWORD!
      ),
      {
        maxConnectionPoolSize: 50,
        connectionAcquisitionTimeout: 30000
      }
    );
  }

  async createInvocationNode(event: Event): Promise<void> {
    const session = this.driver.session();

    try {
      await session.executeWrite(async tx => {
        const result = await tx.run(
          `
          MERGE (inv:Invocation {id: $event_id})
          SET inv.model_id = $model_id,
              inv.provider = $provider,
              inv.timestamp = datetime($timestamp),
              inv.token_count = $token_count,
              inv.latency_ms = $latency_ms,
              inv.status = $status,
              inv.trace_id = $trace_id
          RETURN inv
          `,
          {
            event_id: event.event_id,
            model_id: event.payload.model_id,
            provider: event.payload.provider,
            timestamp: new Date(event.timestamp).toISOString(),
            token_count: event.payload.token_count,
            latency_ms: event.payload.latency_ms,
            status: event.payload.status,
            trace_id: event.trace_id
          }
        );
      });
    } finally {
      await session.close();
    }
  }

  async batchCreateNodes(events: Event[]): Promise<void> {
    const session = this.driver.session();

    try {
      await session.executeWrite(async tx => {
        // Use UNWIND for batch insert
        await tx.run(
          `
          UNWIND $events AS event
          MERGE (inv:Invocation {id: event.event_id})
          SET inv.model_id = event.model_id,
              inv.provider = event.provider,
              inv.timestamp = datetime(event.timestamp),
              inv.token_count = event.token_count,
              inv.latency_ms = event.latency_ms,
              inv.status = event.status,
              inv.trace_id = event.trace_id
          `,
          {
            events: events.map(e => ({
              event_id: e.event_id,
              model_id: e.payload.model_id,
              provider: e.payload.provider,
              timestamp: new Date(e.timestamp).toISOString(),
              token_count: e.payload.token_count,
              latency_ms: e.payload.latency_ms,
              status: e.payload.status,
              trace_id: e.trace_id
            }))
          }
        );
      });
    } finally {
      await session.close();
    }
  }

  async close(): Promise<void> {
    await this.driver.close();
  }
}

// Usage
const config: ObservatoryConfig = {
  brokers: ['kafka-1:9092', 'kafka-2:9092'],
  topic: 'llm-telemetry-events',
  consumerGroup: 'memory-graph-consumers',
  maxBatchSize: 1000,
  bufferSizeMB: 256
};

const consumer = new ObservatoryEventConsumer(config);
await consumer.start();
```

### 1.2 gRPC Client for Observatory Streaming

```typescript
import * as grpc from '@grpc/grpc-js';
import * as protoLoader from '@grpc/proto-loader';
import { EventEmitter } from 'events';

const PROTO_PATH = './proto/observatory.proto';

const packageDefinition = protoLoader.loadSync(PROTO_PATH, {
  keepCase: true,
  longs: String,
  enums: String,
  defaults: true,
  oneofs: true
});

const observatoryProto = grpc.loadPackageDefinition(packageDefinition).llm.observatory.v1;

class ObservatoryStreamClient extends EventEmitter {
  private client: any;
  private stream: grpc.ClientDuplexStream<any, any> | null = null;

  constructor(serverAddress: string) {
    super();

    const credentials = grpc.credentials.createSsl(
      fs.readFileSync('./certs/ca.pem'),
      fs.readFileSync('./certs/client-key.pem'),
      fs.readFileSync('./certs/client.pem')
    );

    this.client = new observatoryProto.TelemetryStream(
      serverAddress,
      credentials
    );
  }

  subscribe(eventTypes: string[]): void {
    const request = {
      event_types: eventTypes,
      buffer_size: 1000,
      filters: {}
    };

    this.stream = this.client.SubscribeEvents(request);

    this.stream.on('data', (event: any) => {
      this.emit('event', event);
    });

    this.stream.on('error', (error: Error) => {
      console.error('Stream error:', error);
      this.emit('error', error);
      this.reconnect(eventTypes);
    });

    this.stream.on('end', () => {
      console.log('Stream ended');
      this.emit('end');
    });
  }

  private reconnect(eventTypes: string[]): void {
    setTimeout(() => {
      console.log('Reconnecting to Observatory stream...');
      this.subscribe(eventTypes);
    }, 5000); // Exponential backoff would be better
  }

  close(): void {
    if (this.stream) {
      this.stream.cancel();
    }
  }
}

// Usage
const streamClient = new ObservatoryStreamClient('observatory.prod:50051');

streamClient.on('event', async (event) => {
  console.log('Received event:', event.event_type);
  // Process event
});

streamClient.on('error', (error) => {
  console.error('Stream error:', error);
});

streamClient.subscribe([
  'llm.invocation.complete',
  'llm.error.occurred',
  'llm.latency.measured'
]);
```

---

## 2. REGISTRY INTEGRATION - CODE EXAMPLES

### 2.1 GraphQL Subscription Client

```typescript
import { createClient, Client } from 'graphql-ws';
import WebSocket from 'ws';
import { GraphQLClient, gql } from 'graphql-request';

class RegistrySubscriptionClient {
  private wsClient: Client;
  private httpClient: GraphQLClient;

  constructor(wsUrl: string, httpUrl: string, authToken: string) {
    this.wsClient = createClient({
      url: wsUrl,
      webSocketImpl: WebSocket,
      connectionParams: {
        authorization: `Bearer ${authToken}`
      },
      retryAttempts: 5,
      retryWait: async (retries) => {
        await new Promise(resolve =>
          setTimeout(resolve, Math.min(1000 * 2 ** retries, 30000))
        );
      }
    });

    this.httpClient = new GraphQLClient(httpUrl, {
      headers: {
        authorization: `Bearer ${authToken}`
      }
    });
  }

  subscribeToModelUpdates(
    callback: (update: ModelUpdate) => void,
    filter?: ModelFilter
  ): () => void {
    const subscription = gql`
      subscription ModelUpdated($filter: ModelFilter) {
        modelUpdated(filter: $filter) {
          updateId
          timestamp
          updateType
          model {
            modelId
            version
            provider
            capabilities
            contextWindow
            trainingCutoff
            metadata
          }
          previousState {
            modelId
            version
          }
          changelog
        }
      }
    `;

    const unsubscribe = this.wsClient.subscribe(
      {
        query: subscription,
        variables: { filter }
      },
      {
        next: (data) => {
          callback(data.data.modelUpdated);
        },
        error: (error) => {
          console.error('Subscription error:', error);
        },
        complete: () => {
          console.log('Subscription completed');
        }
      }
    );

    return () => unsubscribe();
  }

  async fetchModelMetadata(modelId: string): Promise<Model> {
    const query = gql`
      query GetModel($modelId: ID!) {
        model(modelId: $modelId) {
          modelId
          version
          provider
          capabilities
          contextWindow
          trainingCutoff
          metadata
        }
      }
    `;

    const data = await this.httpClient.request(query, { modelId });
    return data.model;
  }

  async reportUsageStats(modelId: string, stats: UsageStats): Promise<void> {
    const mutation = gql`
      mutation ReportUsage($modelId: ID!, $stats: UsageStatsInput!) {
        reportUsageStats(modelId: $modelId, stats: $stats) {
          success
          message
        }
      }
    `;

    await this.httpClient.request(mutation, { modelId, stats });
  }

  close(): void {
    this.wsClient.dispose();
  }
}

// Model Metadata Enrichment Service
class ModelMetadataEnricher {
  private registryClient: RegistrySubscriptionClient;
  private graphWriter: GraphWriter;
  private metadataCache: Map<string, Model>;

  constructor(registryClient: RegistrySubscriptionClient, graphWriter: GraphWriter) {
    this.registryClient = registryClient;
    this.graphWriter = graphWriter;
    this.metadataCache = new Map();
  }

  start(): void {
    // Subscribe to model updates
    this.registryClient.subscribeToModelUpdates(
      this.handleModelUpdate.bind(this),
      {
        updatedAfter: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString() // Last 24h
      }
    );
  }

  private async handleModelUpdate(update: ModelUpdate): Promise<void> {
    console.log(`Model update received: ${update.model.modelId} (${update.updateType})`);

    // Update cache
    this.metadataCache.set(update.model.modelId, update.model);

    // Apply merge strategy based on update type
    switch (update.updateType) {
      case 'CREATED':
        await this.createModelNode(update.model);
        break;
      case 'UPDATED':
        await this.updateModelNode(update.model, update.previousState);
        break;
      case 'DEPRECATED':
        await this.deprecateModelNode(update.model);
        break;
      case 'DELETED':
        await this.softDeleteModelNode(update.model);
        break;
    }
  }

  private async createModelNode(model: Model): Promise<void> {
    const session = this.graphWriter.driver.session();

    try {
      await session.executeWrite(async tx => {
        await tx.run(
          `
          CREATE (m:Model {
            id: $modelId,
            version: $version,
            provider: $provider,
            capabilities: $capabilities,
            context_window: $contextWindow,
            training_cutoff: $trainingCutoff,
            metadata: $metadata,
            created_at: datetime(),
            deprecated: false
          })
          `,
          {
            modelId: model.modelId,
            version: model.version,
            provider: model.provider,
            capabilities: model.capabilities,
            contextWindow: model.contextWindow,
            trainingCutoff: model.trainingCutoff,
            metadata: JSON.stringify(model.metadata)
          }
        );
      });
    } finally {
      await session.close();
    }
  }

  private async updateModelNode(model: Model, previousState: Model): Promise<void> {
    const session = this.graphWriter.driver.session();

    try {
      await session.executeWrite(async tx => {
        // Upsert with latest_wins strategy
        await tx.run(
          `
          MATCH (m:Model {id: $modelId})
          SET m.version = $version,
              m.capabilities = $capabilities,
              m.context_window = $contextWindow,
              m.metadata = $metadata,
              m.updated_at = datetime()

          // Create version history relationship
          WITH m
          CREATE (v:ModelVersion {
            version: $previousVersion,
            deprecated_at: datetime(),
            capabilities: $previousCapabilities
          })
          CREATE (m)-[:HAD_VERSION]->(v)
          `,
          {
            modelId: model.modelId,
            version: model.version,
            capabilities: model.capabilities,
            contextWindow: model.contextWindow,
            metadata: JSON.stringify(model.metadata),
            previousVersion: previousState.version,
            previousCapabilities: previousState.capabilities
          }
        );
      });
    } finally {
      await session.close();
    }
  }

  private async deprecateModelNode(model: Model): Promise<void> {
    const session = this.graphWriter.driver.session();

    try {
      await session.executeWrite(async tx => {
        await tx.run(
          `
          MATCH (m:Model {id: $modelId})
          SET m.deprecated = true,
              m.deprecated_at = datetime()
          `,
          { modelId: model.modelId }
        );
      });
    } finally {
      await session.close();
    }
  }

  private async softDeleteModelNode(model: Model): Promise<void> {
    const session = this.graphWriter.driver.session();

    try {
      await session.executeWrite(async tx => {
        // Don't actually delete, just mark
        await tx.run(
          `
          MATCH (m:Model {id: $modelId})
          SET m.deleted = true,
              m.deleted_at = datetime(),
              m.grace_period_ends = datetime() + duration({days: 30})
          `,
          { modelId: model.modelId }
        );
      });
    } finally {
      await session.close();
    }
  }

  async enrichInvocationWithModelData(invocationId: string): Promise<void> {
    const session = this.graphWriter.driver.session();

    try {
      // First get invocation
      const result = await session.executeRead(async tx => {
        return await tx.run(
          `
          MATCH (inv:Invocation {id: $invocationId})
          RETURN inv.model_id as modelId
          `,
          { invocationId }
        );
      });

      const modelId = result.records[0]?.get('modelId');
      if (!modelId) return;

      // Get model metadata from cache or fetch
      let model = this.metadataCache.get(modelId);
      if (!model) {
        model = await this.registryClient.fetchModelMetadata(modelId);
        this.metadataCache.set(modelId, model);
      }

      // Create relationship
      await session.executeWrite(async tx => {
        await tx.run(
          `
          MATCH (inv:Invocation {id: $invocationId})
          MATCH (m:Model {id: $modelId})
          MERGE (inv)-[:USES_MODEL]->(m)
          `,
          { invocationId, modelId }
        );
      });

    } finally {
      await session.close();
    }
  }
}
```

### 2.2 Change Data Capture (Debezium) Consumer

```typescript
import { Kafka } from 'kafkajs';

interface CDCEvent {
  before: any | null;
  after: any | null;
  op: 'c' | 'u' | 'd' | 'r'; // create, update, delete, read
  ts_ms: number;
  source: {
    table: string;
    db: string;
  };
}

class RegistryCDCConsumer {
  private kafka: Kafka;
  private consumer: any;

  constructor() {
    this.kafka = new Kafka({
      clientId: 'memory-graph-cdc',
      brokers: ['kafka:9092']
    });

    this.consumer = this.kafka.consumer({
      groupId: 'registry-cdc-consumers'
    });
  }

  async start(): Promise<void> {
    await this.consumer.connect();
    await this.consumer.subscribe({
      topic: 'registry.public.models',
      fromBeginning: false
    });

    await this.consumer.run({
      eachMessage: async ({ topic, partition, message }) => {
        const cdcEvent: CDCEvent = JSON.parse(message.value.toString());
        await this.processCDCEvent(cdcEvent);
      }
    });
  }

  private async processCDCEvent(event: CDCEvent): Promise<void> {
    switch (event.op) {
      case 'c': // Create
        await this.handleCreate(event.after);
        break;
      case 'u': // Update
        await this.handleUpdate(event.before, event.after);
        break;
      case 'd': // Delete
        await this.handleDelete(event.before);
        break;
    }
  }

  private async handleCreate(record: any): Promise<void> {
    console.log('New model created in registry:', record.model_id);
    // Trigger sync
  }

  private async handleUpdate(before: any, after: any): Promise<void> {
    console.log('Model updated in registry:', after.model_id);

    // Detect what changed
    const changes = this.detectChanges(before, after);
    console.log('Changes detected:', changes);

    // Apply merge strategy based on changes
  }

  private async handleDelete(record: any): Promise<void> {
    console.log('Model deleted in registry:', record.model_id);
    // Trigger soft delete
  }

  private detectChanges(before: any, after: any): string[] {
    const changes: string[] = [];

    for (const key of Object.keys(after)) {
      if (JSON.stringify(before[key]) !== JSON.stringify(after[key])) {
        changes.push(key);
      }
    }

    return changes;
  }
}
```

---

## 3. DATA VAULT INTEGRATION - CODE EXAMPLES

### 3.1 Secure Storage Client with Encryption

```typescript
import * as crypto from 'crypto';
import axios from 'axios';
import { KMS } from '@aws-sdk/client-kms';

interface EncryptionResult {
  encryptedContent: Buffer;
  encryptedDEK: Buffer;
  iv: Buffer;
  authTag: Buffer;
  kekId: string;
}

class DataVaultClient {
  private vaultUrl: string;
  private kms: KMS;
  private kekId: string;
  private axiosInstance: any;

  constructor(vaultUrl: string, kekId: string) {
    this.vaultUrl = vaultUrl;
    this.kekId = kekId;

    this.kms = new KMS({
      region: process.env.AWS_REGION || 'us-east-1'
    });

    // Configure Axios with mTLS
    this.axiosInstance = axios.create({
      baseURL: vaultUrl,
      httpsAgent: new https.Agent({
        cert: fs.readFileSync('./certs/client.pem'),
        key: fs.readFileSync('./certs/client-key.pem'),
        ca: fs.readFileSync('./certs/ca.pem'),
        rejectUnauthorized: true
      }),
      headers: {
        'Content-Type': 'application/json'
      }
    });
  }

  async storeContent(
    contentId: string,
    content: string,
    classification: string,
    metadata: Record<string, string> = {}
  ): Promise<string> {
    // 1. Encrypt content locally
    const encrypted = await this.encryptContent(content);

    // 2. Send to vault
    const response = await this.axiosInstance.post('/api/v1/vault/store', {
      content_id: contentId,
      encrypted_content: encrypted.encryptedContent.toString('base64'),
      encrypted_dek: encrypted.encryptedDEK.toString('base64'),
      iv: encrypted.iv.toString('base64'),
      auth_tag: encrypted.authTag.toString('base64'),
      kek_id: encrypted.kekId,
      classification,
      metadata,
      encryption_algorithm: 'AES-256-GCM'
    });

    return response.data.vault_reference;
  }

  async retrieveContent(
    contentId: string,
    requesterId: string,
    accessReason: string
  ): Promise<string> {
    // 1. Request from vault with audit info
    const response = await this.axiosInstance.get(
      `/api/v1/vault/retrieve/${contentId}`,
      {
        headers: {
          'X-Requester-Id': requesterId,
          'X-Access-Reason': accessReason
        }
      }
    );

    const {
      encrypted_content,
      encrypted_dek,
      iv,
      auth_tag
    } = response.data;

    // 2. Decrypt locally
    const content = await this.decryptContent({
      encryptedContent: Buffer.from(encrypted_content, 'base64'),
      encryptedDEK: Buffer.from(encrypted_dek, 'base64'),
      iv: Buffer.from(iv, 'base64'),
      authTag: Buffer.from(auth_tag, 'base64'),
      kekId: this.kekId
    });

    return content;
  }

  private async encryptContent(plaintext: string): Promise<EncryptionResult> {
    // 1. Generate Data Encryption Key (DEK)
    const dek = crypto.randomBytes(32); // 256-bit key

    // 2. Encrypt content with DEK
    const iv = crypto.randomBytes(16);
    const cipher = crypto.createCipheriv('aes-256-gcm', dek, iv);

    let encryptedContent = cipher.update(plaintext, 'utf8');
    encryptedContent = Buffer.concat([encryptedContent, cipher.final()]);
    const authTag = cipher.getAuthTag();

    // 3. Encrypt DEK with KEK from KMS
    const kmsResponse = await this.kms.encrypt({
      KeyId: this.kekId,
      Plaintext: dek
    });

    const encryptedDEK = Buffer.from(kmsResponse.CiphertextBlob!);

    return {
      encryptedContent,
      encryptedDEK,
      iv,
      authTag,
      kekId: this.kekId
    };
  }

  private async decryptContent(encrypted: EncryptionResult): Promise<string> {
    // 1. Decrypt DEK with KMS
    const kmsResponse = await this.kms.decrypt({
      CiphertextBlob: encrypted.encryptedDEK,
      KeyId: encrypted.kekId
    });

    const dek = Buffer.from(kmsResponse.Plaintext!);

    // 2. Decrypt content with DEK
    const decipher = crypto.createDecipheriv('aes-256-gcm', dek, encrypted.iv);
    decipher.setAuthTag(encrypted.authTag);

    let decrypted = decipher.update(encrypted.encryptedContent);
    decrypted = Buffer.concat([decrypted, decipher.final()]);

    return decrypted.toString('utf8');
  }

  async deleteContent(
    contentId: string,
    deletionReason: string,
    requesterId: string
  ): Promise<void> {
    await this.axiosInstance.delete(`/api/v1/vault/delete/${contentId}`, {
      data: {
        deletion_reason: deletionReason,
        requester_id: requesterId
      }
    });
  }
}

// Integration with Memory Graph
class SecurePromptStorage {
  private vaultClient: DataVaultClient;
  private graphWriter: GraphWriter;

  constructor(vaultClient: DataVaultClient, graphWriter: GraphWriter) {
    this.vaultClient = vaultClient;
    this.graphWriter = graphWriter;
  }

  async storePromptSecurely(
    promptId: string,
    promptContent: string,
    userId: string,
    classification: string = 'confidential'
  ): Promise<void> {
    // 1. Store content in vault
    const vaultRef = await this.vaultClient.storeContent(
      promptId,
      promptContent,
      classification,
      {
        user_id: userId,
        content_type: 'prompt',
        stored_at: new Date().toISOString()
      }
    );

    // 2. Store reference in graph
    const session = this.graphWriter.driver.session();

    try {
      await session.executeWrite(async tx => {
        await tx.run(
          `
          CREATE (p:Prompt {
            id: $promptId,
            content_ref: $vaultRef,
            classification: $classification,
            user_id: $userId,
            created_at: datetime(),
            has_sensitive_content: true
          })
          `,
          {
            promptId,
            vaultRef,
            classification,
            userId
          }
        );
      });
    } finally {
      await session.close();
    }
  }

  async retrievePromptSecurely(
    promptId: string,
    requesterId: string,
    accessReason: string
  ): Promise<string> {
    // 1. Get vault reference from graph
    const session = this.graphWriter.driver.session();
    let vaultRef: string;

    try {
      const result = await session.executeRead(async tx => {
        return await tx.run(
          `
          MATCH (p:Prompt {id: $promptId})
          RETURN p.content_ref as vaultRef, p.classification as classification
          `,
          { promptId }
        );
      });

      if (result.records.length === 0) {
        throw new Error('Prompt not found');
      }

      vaultRef = result.records[0].get('vaultRef');
    } finally {
      await session.close();
    }

    // 2. Retrieve from vault with audit trail
    const content = await this.vaultClient.retrieveContent(
      promptId,
      requesterId,
      accessReason
    );

    return content;
  }
}
```

### 3.2 Access Control Middleware

```typescript
import { Request, Response, NextFunction } from 'express';
import * as jwt from 'jsonwebtoken';

interface AccessPolicy {
  allowedRoles: string[];
  allowedUsers: string[];
  conditions: Record<string, any>;
}

interface User {
  userId: string;
  roles: string[];
  clearanceLevel: number;
  mfaVerified: boolean;
}

class AccessControlMiddleware {
  private policyEngine: ABACPolicyEngine;

  constructor(policyEngine: ABACPolicyEngine) {
    this.policyEngine = policyEngine;
  }

  checkAccess(requiredClassification: string) {
    return async (req: Request, res: Response, next: NextFunction) => {
      try {
        // 1. Authenticate user
        const token = req.headers.authorization?.split(' ')[1];
        if (!token) {
          return res.status(401).json({ error: 'No token provided' });
        }

        const user = this.verifyToken(token);

        // 2. Check RBAC
        if (!this.checkRBAC(user, requiredClassification)) {
          await this.auditAccessDenied(user, req, 'RBAC check failed');
          return res.status(403).json({ error: 'Access denied: Insufficient role' });
        }

        // 3. Check ABAC
        const abacResult = await this.policyEngine.evaluate({
          user,
          resource: {
            classification: requiredClassification,
            contentId: req.params.contentId
          },
          context: {
            time: new Date(),
            sourceNetwork: req.ip,
            userAgent: req.headers['user-agent']
          }
        });

        if (!abacResult.allowed) {
          await this.auditAccessDenied(user, req, `ABAC check failed: ${abacResult.reason}`);
          return res.status(403).json({
            error: 'Access denied',
            reason: abacResult.reason
          });
        }

        // 4. Check MFA for restricted data
        if (requiredClassification === 'restricted' && !user.mfaVerified) {
          return res.status(403).json({
            error: 'MFA required for restricted data'
          });
        }

        // Attach user to request
        req.user = user;
        next();

      } catch (error) {
        console.error('Access control error:', error);
        res.status(500).json({ error: 'Internal server error' });
      }
    };
  }

  private verifyToken(token: string): User {
    const decoded = jwt.verify(token, process.env.JWT_SECRET!) as any;

    return {
      userId: decoded.sub,
      roles: decoded.roles || [],
      clearanceLevel: decoded.clearance_level || 0,
      mfaVerified: decoded.mfa_verified || false
    };
  }

  private checkRBAC(user: User, classification: string): boolean {
    const rolePermissions: Record<string, string[]> = {
      'admin': ['public', 'internal', 'confidential', 'restricted', 'pii'],
      'compliance_officer': ['public', 'internal', 'confidential', 'restricted', 'pii'],
      'ml_engineer': ['public', 'internal', 'confidential'],
      'data_scientist': ['public', 'internal']
    };

    for (const role of user.roles) {
      const permissions = rolePermissions[role] || [];
      if (permissions.includes(classification)) {
        return true;
      }
    }

    return false;
  }

  private async auditAccessDenied(
    user: User,
    req: Request,
    reason: string
  ): Promise<void> {
    const auditLog = {
      audit_id: crypto.randomUUID(),
      timestamp: new Date().toISOString(),
      user_id: user.userId,
      action: 'access_denied',
      resource: req.path,
      reason,
      source_ip: req.ip,
      user_agent: req.headers['user-agent']
    };

    // Send to audit log service
    await this.sendAuditLog(auditLog);
  }

  private async sendAuditLog(log: any): Promise<void> {
    // Send to Kafka, Elasticsearch, or audit database
    console.log('Audit log:', log);
  }
}

// ABAC Policy Engine
class ABACPolicyEngine {
  async evaluate(context: {
    user: User;
    resource: any;
    context: any;
  }): Promise<{ allowed: boolean; reason?: string }> {
    // PII access policy
    if (context.resource.classification === 'pii') {
      const allowedRoles = ['compliance_officer', 'admin'];
      const hasRole = context.user.roles.some(r => allowedRoles.includes(r));

      if (!hasRole) {
        return { allowed: false, reason: 'PII access requires compliance_officer or admin role' };
      }

      // Check time constraint (9 AM - 5 PM)
      const hour = context.context.time.getHours();
      if (hour < 9 || hour >= 17) {
        return { allowed: false, reason: 'PII access only allowed during business hours (9 AM - 5 PM)' };
      }
    }

    // Restricted data policy
    if (context.resource.classification === 'restricted') {
      if (!context.user.mfaVerified) {
        return { allowed: false, reason: 'MFA required for restricted data' };
      }

      if (context.user.clearanceLevel < 3) {
        return { allowed: false, reason: 'Insufficient clearance level for restricted data' };
      }
    }

    return { allowed: true };
  }
}
```

---

## 4. OBSERVABILITY IMPLEMENTATION

### 4.1 OpenTelemetry Integration

```typescript
import { NodeSDK } from '@opentelemetry/sdk-node';
import { getNodeAutoInstrumentations } from '@opentelemetry/auto-instrumentations-node';
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-grpc';
import { OTLPMetricExporter } from '@opentelemetry/exporter-metrics-otlp-grpc';
import { PeriodicExportingMetricReader } from '@opentelemetry/sdk-metrics';
import { Resource } from '@opentelemetry/resources';
import { SemanticResourceAttributes } from '@opentelemetry/semantic-conventions';

const sdk = new NodeSDK({
  resource: new Resource({
    [SemanticResourceAttributes.SERVICE_NAME]: 'llm-memory-graph',
    [SemanticResourceAttributes.SERVICE_VERSION]: process.env.APP_VERSION,
    [SemanticResourceAttributes.DEPLOYMENT_ENVIRONMENT]: process.env.ENVIRONMENT
  }),
  traceExporter: new OTLPTraceExporter({
    url: 'grpc://otel-collector:4317'
  }),
  metricReader: new PeriodicExportingMetricReader({
    exporter: new OTLPMetricExporter({
      url: 'grpc://otel-collector:4317'
    }),
    exportIntervalMillis: 10000
  }),
  instrumentations: [
    getNodeAutoInstrumentations({
      '@opentelemetry/instrumentation-fs': { enabled: false }
    })
  ]
});

sdk.start();

// Graceful shutdown
process.on('SIGTERM', () => {
  sdk.shutdown()
    .then(() => console.log('Tracing terminated'))
    .catch((error) => console.error('Error terminating tracing', error))
    .finally(() => process.exit(0));
});

// Custom instrumentation
import { trace, context, SpanStatusCode } from '@opentelemetry/api';

const tracer = trace.getTracer('llm-memory-graph', '1.0.0');

async function processEventWithTracing(event: Event): Promise<void> {
  const span = tracer.startSpan('process_event', {
    attributes: {
      'event.id': event.event_id,
      'event.type': event.event_type,
      'event.trace_id': event.trace_id
    }
  });

  try {
    // Process event
    await processEvent(event);

    span.setStatus({ code: SpanStatusCode.OK });
  } catch (error) {
    span.setStatus({
      code: SpanStatusCode.ERROR,
      message: error.message
    });
    span.recordException(error);
    throw error;
  } finally {
    span.end();
  }
}
```

---

## 5. DEPLOYMENT CONFIGURATIONS

### 5.1 Kubernetes Deployment with Secrets

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-memory-graph
  namespace: llm-platform
spec:
  replicas: 3
  selector:
    matchLabels:
      app: llm-memory-graph
  template:
    metadata:
      labels:
        app: llm-memory-graph
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: llm-memory-graph-sa

      # Init container for certificate setup
      initContainers:
      - name: cert-setup
        image: busybox
        command:
        - sh
        - -c
        - |
          cp /tmp/certs/* /etc/ssl/certs/
          chmod 600 /etc/ssl/certs/*.pem
        volumeMounts:
        - name: certs
          mountPath: /tmp/certs
        - name: ssl-certs
          mountPath: /etc/ssl/certs

      containers:
      - name: memory-graph
        image: llm-memory-graph:1.0.0
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 50051
          name: grpc
        - containerPort: 9090
          name: metrics

        env:
        # Observatory Integration
        - name: KAFKA_BROKERS
          value: "kafka-headless:9092"
        - name: KAFKA_USERNAME
          valueFrom:
            secretKeyRef:
              name: kafka-credentials
              key: username
        - name: KAFKA_PASSWORD
          valueFrom:
            secretKeyRef:
              name: kafka-credentials
              key: password

        # Registry Integration
        - name: REGISTRY_API_URL
          value: "https://llm-registry:8080"
        - name: REGISTRY_CLIENT_ID
          valueFrom:
            secretKeyRef:
              name: registry-credentials
              key: client_id
        - name: REGISTRY_CLIENT_SECRET
          valueFrom:
            secretKeyRef:
              name: registry-credentials
              key: client_secret

        # Vault Integration
        - name: VAULT_API_URL
          value: "https://llm-vault:8443"
        - name: AWS_REGION
          value: "us-east-1"
        - name: KMS_KEY_ID
          valueFrom:
            secretKeyRef:
              name: vault-credentials
              key: kms_key_id

        # Graph Database
        - name: NEO4J_URI
          value: "bolt://neo4j-cluster:7687"
        - name: NEO4J_USERNAME
          valueFrom:
            secretKeyRef:
              name: neo4j-credentials
              key: username
        - name: NEO4J_PASSWORD
          valueFrom:
            secretKeyRef:
              name: neo4j-credentials
              key: password

        # Observability
        - name: OTEL_EXPORTER_OTLP_ENDPOINT
          value: "grpc://otel-collector:4317"

        resources:
          requests:
            cpu: "1"
            memory: "2Gi"
          limits:
            cpu: "4"
            memory: "8Gi"

        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10

        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5

        volumeMounts:
        - name: ssl-certs
          mountPath: /etc/ssl/certs
          readOnly: true
        - name: config
          mountPath: /etc/memory-graph
          readOnly: true

      volumes:
      - name: certs
        secret:
          secretName: llm-memory-graph-certs
      - name: ssl-certs
        emptyDir: {}
      - name: config
        configMap:
          name: llm-memory-graph-config

---
apiVersion: v1
kind: Service
metadata:
  name: llm-memory-graph
  namespace: llm-platform
spec:
  selector:
    app: llm-memory-graph
  ports:
  - name: http
    port: 8080
    targetPort: 8080
  - name: grpc
    port: 50051
    targetPort: 50051
  - name: metrics
    port: 9090
    targetPort: 9090
  type: ClusterIP

---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: llm-memory-graph-hpa
  namespace: llm-platform
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: llm-memory-graph
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  - type: Pods
    pods:
      metric:
        name: kafka_consumer_lag
      target:
        type: AverageValue
        averageValue: "5000"
```

### 5.2 Service Mesh (Istio) Configuration

```yaml
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: llm-memory-graph
  namespace: llm-platform
spec:
  hosts:
  - llm-memory-graph
  http:
  - match:
    - headers:
        x-api-version:
          exact: v1
    route:
    - destination:
        host: llm-memory-graph
        subset: v1
  - route:
    - destination:
        host: llm-memory-graph
        subset: v1

---
apiVersion: networking.istio.io/v1beta1
kind: DestinationRule
metadata:
  name: llm-memory-graph
  namespace: llm-platform
spec:
  host: llm-memory-graph
  trafficPolicy:
    tls:
      mode: ISTIO_MUTUAL
    connectionPool:
      tcp:
        maxConnections: 100
      http:
        http1MaxPendingRequests: 50
        http2MaxRequests: 100
        maxRequestsPerConnection: 2
    outlierDetection:
      consecutiveErrors: 5
      interval: 30s
      baseEjectionTime: 30s
      maxEjectionPercent: 50
  subsets:
  - name: v1
    labels:
      version: v1

---
apiVersion: security.istio.io/v1beta1
kind: PeerAuthentication
metadata:
  name: llm-memory-graph
  namespace: llm-platform
spec:
  selector:
    matchLabels:
      app: llm-memory-graph
  mtls:
    mode: STRICT
```

---

## 6. TESTING EXAMPLES

### 6.1 Integration Test Suite

```typescript
import { describe, it, expect, beforeAll, afterAll } from '@jest/globals';
import { ObservatoryEventConsumer } from './observatory-consumer';
import { DataVaultClient } from './vault-client';
import { GraphWriter } from './graph-writer';

describe('End-to-End Integration Tests', () => {
  let consumer: ObservatoryEventConsumer;
  let vaultClient: DataVaultClient;
  let graphWriter: GraphWriter;

  beforeAll(async () => {
    // Setup test environment
    consumer = new ObservatoryEventConsumer(testConfig);
    vaultClient = new DataVaultClient(testVaultUrl, testKekId);
    graphWriter = new GraphWriter();
  });

  afterAll(async () => {
    await consumer.shutdown();
    await graphWriter.close();
  });

  describe('Observatory Event Processing', () => {
    it('should process high-volume event stream without data loss', async () => {
      const eventsProduced = 10000;
      const startTime = Date.now();

      // Produce events
      await produceTestEvents(eventsProduced);

      // Wait for processing
      await waitForProcessing(eventsProduced, 30000);

      // Verify all events in graph
      const eventCount = await graphWriter.countEvents();
      expect(eventCount).toBe(eventsProduced);

      const duration = Date.now() - startTime;
      const throughput = eventsProduced / (duration / 1000);

      console.log(`Throughput: ${throughput.toFixed(2)} events/sec`);
      expect(throughput).toBeGreaterThan(100);
    });

    it('should handle backpressure correctly', async () => {
      // Overwhelm consumer
      const eventsProduced = 50000;
      await produceTestEvents(eventsProduced, 100); // 100ms batch

      // Check circuit breaker activation
      const cbStatus = await consumer.getCircuitBreakerStatus();
      expect(cbStatus.activated).toBe(true);

      // Verify no memory overflow
      const memUsage = process.memoryUsage();
      expect(memUsage.heapUsed).toBeLessThan(8 * 1024 * 1024 * 1024); // 8GB
    });
  });

  describe('Vault Integration', () => {
    it('should encrypt and decrypt content correctly', async () => {
      const testContent = 'This is sensitive prompt data';
      const contentId = 'test-prompt-001';

      // Store
      const vaultRef = await vaultClient.storeContent(
        contentId,
        testContent,
        'confidential'
      );

      expect(vaultRef).toBeTruthy();

      // Retrieve
      const retrieved = await vaultClient.retrieveContent(
        contentId,
        'test-user',
        'Integration test'
      );

      expect(retrieved).toBe(testContent);
    });

    it('should enforce access control', async () => {
      const contentId = 'restricted-prompt-001';

      await vaultClient.storeContent(
        contentId,
        'Highly sensitive data',
        'restricted'
      );

      // Attempt unauthorized access
      await expect(
        vaultClient.retrieveContent(contentId, 'unauthorized-user', 'Test')
      ).rejects.toThrow('Access denied');
    });
  });

  describe('Registry Synchronization', () => {
    it('should sync model metadata changes', async () => {
      const modelId = 'test-model-001';

      // Trigger registry update
      await updateModelInRegistry(modelId, {
        capabilities: ['chat', 'vision', 'tools']
      });

      // Wait for sync
      await waitForSync(5000);

      // Verify in graph
      const model = await graphWriter.getModel(modelId);
      expect(model.capabilities).toContain('tools');
    });

    it('should handle schema evolution', async () => {
      // Deploy new schema version
      await deployNewSchema('v2.0.0');

      // Verify backward compatibility
      const oldFormatEvent = createLegacyEvent();
      await consumer.handleMessage({ message: oldFormatEvent });

      // Should not throw
      expect(true).toBe(true);
    });
  });
});
```

---

**Document Version:** 1.0
**Last Updated:** 2025-11-06
**Companion to:** INTEGRATION_ARCHITECTURE.md
