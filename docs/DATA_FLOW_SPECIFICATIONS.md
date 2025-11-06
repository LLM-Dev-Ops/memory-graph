# LLM-Memory-Graph Data Flow Specifications

## 1. EVENT SCHEMAS

### 1.1 Inbound Events (from LLM-Observatory)

#### PromptEvent Schema
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["eventId", "timestamp", "prompt", "session", "model"],
  "properties": {
    "eventId": {
      "type": "string",
      "format": "uuid",
      "description": "Unique event identifier"
    },
    "timestamp": {
      "type": "string",
      "format": "date-time",
      "description": "ISO 8601 timestamp"
    },
    "prompt": {
      "type": "object",
      "required": ["text"],
      "properties": {
        "text": {
          "type": "string",
          "minLength": 1,
          "maxLength": 1000000,
          "description": "The prompt text"
        },
        "template": {
          "type": "string",
          "description": "Extracted template with {{variables}}"
        },
        "parameters": {
          "type": "object",
          "description": "Template parameter values",
          "additionalProperties": true
        },
        "images": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "url": {"type": "string", "format": "uri"},
              "base64": {"type": "string"},
              "mimeType": {"type": "string"}
            }
          },
          "description": "Multi-modal image inputs"
        },
        "metadata": {
          "type": "object",
          "additionalProperties": true
        }
      }
    },
    "session": {
      "type": "object",
      "required": ["id", "userId"],
      "properties": {
        "id": {"type": "string"},
        "userId": {"type": "string"},
        "applicationId": {"type": "string"},
        "purpose": {
          "type": "string",
          "enum": ["chat", "completion", "embedding", "fine_tune", "other"]
        }
      }
    },
    "model": {
      "type": "object",
      "required": ["id"],
      "properties": {
        "id": {"type": "string"},
        "provider": {"type": "string"},
        "version": {"type": "string"},
        "parameters": {
          "type": "object",
          "properties": {
            "temperature": {"type": "number", "minimum": 0, "maximum": 2},
            "topP": {"type": "number", "minimum": 0, "maximum": 1},
            "topK": {"type": "integer", "minimum": 1},
            "maxTokens": {"type": "integer", "minimum": 1},
            "stopSequences": {"type": "array", "items": {"type": "string"}},
            "presencePenalty": {"type": "number", "minimum": -2, "maximum": 2},
            "frequencyPenalty": {"type": "number", "minimum": -2, "maximum": 2}
          }
        }
      }
    },
    "context": {
      "type": "object",
      "properties": {
        "conversationHistory": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "role": {"type": "string", "enum": ["user", "assistant", "system"]},
              "content": {"type": "string"}
            }
          }
        },
        "documents": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "id": {"type": "string"},
              "content": {"type": "string"},
              "relevanceScore": {"type": "number"}
            }
          }
        },
        "retrievalMethod": {
          "type": "string",
          "enum": ["embedding", "keyword", "hybrid", "none"]
        }
      }
    },
    "tags": {
      "type": "array",
      "items": {"type": "string"}
    },
    "traceContext": {
      "type": "object",
      "properties": {
        "traceId": {"type": "string"},
        "spanId": {"type": "string"},
        "parentSpanId": {"type": "string"}
      }
    }
  }
}
```

#### ResponseEvent Schema
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["eventId", "timestamp", "promptEventId", "response"],
  "properties": {
    "eventId": {
      "type": "string",
      "format": "uuid"
    },
    "timestamp": {
      "type": "string",
      "format": "date-time"
    },
    "promptEventId": {
      "type": "string",
      "format": "uuid",
      "description": "References the prompt that generated this response"
    },
    "response": {
      "type": "object",
      "properties": {
        "text": {
          "type": "string",
          "description": "The generated response text"
        },
        "finishReason": {
          "type": "string",
          "enum": ["stop", "length", "content_filter", "function_call", "error"]
        },
        "functionCall": {
          "type": "object",
          "properties": {
            "name": {"type": "string"},
            "arguments": {"type": "string"}
          }
        },
        "toolCalls": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "id": {"type": "string"},
              "type": {"type": "string"},
              "function": {
                "type": "object",
                "properties": {
                  "name": {"type": "string"},
                  "arguments": {"type": "string"}
                }
              }
            }
          }
        }
      }
    },
    "usage": {
      "type": "object",
      "required": ["promptTokens", "completionTokens", "totalTokens"],
      "properties": {
        "promptTokens": {"type": "integer", "minimum": 0},
        "completionTokens": {"type": "integer", "minimum": 0},
        "totalTokens": {"type": "integer", "minimum": 0}
      }
    },
    "metrics": {
      "type": "object",
      "properties": {
        "latency": {
          "type": "integer",
          "description": "Total latency in milliseconds"
        },
        "ttft": {
          "type": "integer",
          "description": "Time to first token in milliseconds"
        },
        "throughput": {
          "type": "number",
          "description": "Tokens per second"
        },
        "cost": {
          "type": "number",
          "description": "Estimated cost in USD"
        }
      }
    },
    "error": {
      "type": "object",
      "properties": {
        "code": {"type": "string"},
        "message": {"type": "string"},
        "type": {"type": "string"},
        "recoverable": {"type": "boolean"}
      }
    },
    "tags": {
      "type": "array",
      "items": {"type": "string"}
    },
    "traceContext": {
      "type": "object",
      "properties": {
        "traceId": {"type": "string"},
        "spanId": {"type": "string"},
        "parentSpanId": {"type": "string"}
      }
    }
  }
}
```

---

## 2. DATA TRANSFORMATION PIPELINE

### 2.1 Enrichment Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                   Enrichment Pipeline Stages                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Stage 1: Validation                                            │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Input: Raw event from Kafka                            │  │
│  │  Actions:                                                │  │
│  │  - Validate against JSON Schema                         │  │
│  │  - Check required fields                                │  │
│  │  - Validate data types                                  │  │
│  │  - Verify timestamp is within acceptable range          │  │
│  │  Output: Valid event OR error                           │  │
│  └──────────────────────────────────────────────────────────┘  │
│                           │                                     │
│                           ▼                                     │
│  Stage 2: Deduplication                                         │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Input: Valid event                                      │  │
│  │  Actions:                                                │  │
│  │  - Check Redis cache for eventId (TTL: 1 hour)          │  │
│  │  - If duplicate, skip processing                        │  │
│  │  - If new, add to cache and continue                    │  │
│  │  Output: Unique event                                   │  │
│  └──────────────────────────────────────────────────────────┘  │
│                           │                                     │
│                           ▼                                     │
│  Stage 3: Text Processing                                       │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Input: Unique event                                     │  │
│  │  Actions:                                                │  │
│  │  - Compute SHA-256 hash of text                          │  │
│  │  - Detect language (langdetect)                          │  │
│  │  - Count tokens (tiktoken)                               │  │
│  │  - Extract template if pattern detected                 │  │
│  │  - Normalize whitespace                                  │  │
│  │  Output: Event + text metadata                          │  │
│  └──────────────────────────────────────────────────────────┘  │
│                           │                                     │
│                           ▼                                     │
│  Stage 4: PII Detection                                         │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Input: Event + text metadata                            │  │
│  │  Actions:                                                │  │
│  │  - Run regex patterns (email, phone, SSN, credit card)  │  │
│  │  - Run NER model (spaCy) for PERSON, ORG, GPE           │  │
│  │  - Flag if PII detected                                 │  │
│  │  - Optionally redact or encrypt                         │  │
│  │  Output: Event + PII flags                              │  │
│  └──────────────────────────────────────────────────────────┘  │
│                           │                                     │
│                           ▼                                     │
│  Stage 5: Embedding Generation                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Input: Event + PII flags                                │  │
│  │  Actions:                                                │  │
│  │  - Generate embedding vector (OpenAI, Cohere, or local) │  │
│  │  - Cache embeddings (Redis) for similar text            │  │
│  │  - Dimension reduction if needed                        │  │
│  │  Output: Event + embedding vector                       │  │
│  └──────────────────────────────────────────────────────────┘  │
│                           │                                     │
│                           ▼                                     │
│  Stage 6: Metadata Lookup                                       │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Input: Event + embedding vector                         │  │
│  │  Actions:                                                │  │
│  │  - Lookup model info from Registry (gRPC call)          │  │
│  │  - Lookup user info from auth service                   │  │
│  │  - Compute estimated cost based on token count          │  │
│  │  - Add tenant/org information                           │  │
│  │  Output: Fully enriched event                           │  │
│  └──────────────────────────────────────────────────────────┘  │
│                           │                                     │
│                           ▼                                     │
│  Stage 7: Graph Extraction                                      │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Input: Fully enriched event                             │  │
│  │  Actions:                                                │  │
│  │  - Extract node properties                              │  │
│  │  - Identify edge relationships                          │  │
│  │  - Detect reuse patterns (similar prompts)              │  │
│  │  - Link to existing session/user nodes                  │  │
│  │  Output: Graph mutation commands                        │  │
│  └──────────────────────────────────────────────────────────┘  │
│                           │                                     │
│                           ▼                                     │
│  Stage 8: Parallel Write                                        │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Input: Graph mutation commands                          │  │
│  │  Actions: (Parallel writes)                              │  │
│  │  - Write to Graph DB (Neo4j)                             │  │
│  │  - Write to Vector Store (Pinecone)                      │  │
│  │  - Write to Time Series DB (InfluxDB)                    │  │
│  │  - Write to Full-Text Index (Elasticsearch)             │  │
│  │  Output: Write confirmations                            │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Error Handling

```
┌─────────────────────────────────────────────────────────────────┐
│                     Error Handling Strategy                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Validation Errors:                                             │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  - Log error with full event payload                     │  │
│  │  - Send to dead letter queue (DLQ)                       │  │
│  │  - Alert if error rate > 1%                              │  │
│  │  - Manual review and reprocessing                        │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Transient Errors (network, timeout):                           │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  - Retry with exponential backoff (3 attempts)           │  │
│  │  - Initial delay: 1s, 2s, 4s                             │  │
│  │  - If all retries fail, send to DLQ                      │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Enrichment Service Unavailable:                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  - Skip optional enrichments (embeddings)                │  │
│  │  - Process with partial data                             │  │
│  │  - Schedule async enrichment job                         │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Write Failures:                                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  - Retry individual write operations                     │  │
│  │  - If Graph DB fails but Vector Store succeeds:          │  │
│  │    * Store in reconciliation queue                       │  │
│  │    * Background job retries later                        │  │
│  │  - Eventual consistency acceptable                       │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. QUERY PROCESSING FLOW

### 3.1 Query Execution Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    Query Execution Flow                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Client Request                                                 │
│       │                                                         │
│       ▼                                                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  API Gateway                                            │   │
│  │  - Authentication (JWT validation)                      │   │
│  │  - Rate limiting (per user/tenant)                      │   │
│  │  - Request validation                                   │   │
│  └─────────────────┬───────────────────────────────────────┘   │
│                    │                                            │
│                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Query Parser                                           │   │
│  │  - Parse GraphQL/Cypher/REST                            │   │
│  │  - Extract intent and parameters                        │   │
│  │  - Validate syntax                                      │   │
│  └─────────────────┬───────────────────────────────────────┘   │
│                    │                                            │
│                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Authorization Check                                    │   │
│  │  - RBAC: Check user permissions                         │   │
│  │  - ABAC: Evaluate attribute-based policies              │   │
│  │  - Row-level security (filter by tenant)                │   │
│  └─────────────────┬───────────────────────────────────────┘   │
│                    │                                            │
│                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Cache Lookup (L1: In-Memory)                           │   │
│  │  - Check local cache for identical query                │   │
│  │  - If hit, return immediately                           │   │
│  └─────────────────┬───────────────────────────────────────┘   │
│                    │ (Cache Miss)                               │
│                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Cache Lookup (L2: Redis)                               │   │
│  │  - Check distributed cache                              │   │
│  │  - If hit, return + populate L1                         │   │
│  └─────────────────┬───────────────────────────────────────┘   │
│                    │ (Cache Miss)                               │
│                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Query Planner                                          │   │
│  │  - Analyze query complexity                             │   │
│  │  - Choose optimal execution strategy:                   │   │
│  │    * Direct graph traversal                             │   │
│  │    * Vector search + filter                             │   │
│  │    * Full-text search + join                            │   │
│  │    * Materialized view lookup                           │   │
│  │  - Estimate cost (time, resources)                      │   │
│  └─────────────────┬───────────────────────────────────────┘   │
│                    │                                            │
│                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Query Router                                           │   │
│  │  - Route to appropriate storage backend(s)              │   │
│  │  - Parallel execution if multiple sources               │   │
│  │  - Shard selection (if partitioned)                     │   │
│  └─────────────────┬───────────────────────────────────────┘   │
│                    │                                            │
│       ┌────────────┼────────────┬────────────┐                 │
│       │            │            │            │                 │
│       ▼            ▼            ▼            ▼                 │
│  ┌────────┐  ┌────────┐  ┌─────────┐  ┌────────┐             │
│  │ Graph  │  │ Vector │  │  Full   │  │  Time  │             │
│  │   DB   │  │ Store  │  │  Text   │  │ Series │             │
│  └───┬────┘  └───┬────┘  └────┬────┘  └───┬────┘             │
│      │           │            │           │                    │
│      └───────────┼────────────┼───────────┘                    │
│                  │            │                                │
│                  ▼            ▼                                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Result Aggregator                                      │   │
│  │  - Merge results from multiple sources                  │   │
│  │  - Apply post-filters                                   │   │
│  │  - Sort and paginate                                    │   │
│  └─────────────────┬───────────────────────────────────────┘   │
│                    │                                            │
│                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Result Enricher                                        │   │
│  │  - Fetch additional metadata if needed                  │   │
│  │  - Format response (JSON, GraphML, etc.)                │   │
│  │  - Add pagination cursors                               │   │
│  └─────────────────┬───────────────────────────────────────┘   │
│                    │                                            │
│                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Cache Population                                       │   │
│  │  - Store result in Redis (TTL: 5-15 min)                │   │
│  │  - Store in L1 cache                                    │   │
│  └─────────────────┬───────────────────────────────────────┘   │
│                    │                                            │
│                    ▼                                            │
│              Return to Client                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Query Optimization Strategies

**Index Selection:**
```
┌─────────────────────────────────────────────────────────────────┐
│                      Index Strategy                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Primary Indexes (Graph DB):                                    │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  - Node.id (unique, clustered)                           │  │
│  │  - Node.textHash (for deduplication)                     │  │
│  │  - Node.createdAt (for time-range queries)               │  │
│  │  - Node.userId (for user-specific queries)               │  │
│  │  - Node.sessionId (for session queries)                  │  │
│  │  - Edge.(from, to) (composite, for traversals)           │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Full-Text Indexes (Elasticsearch):                             │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  - Prompt.text (analyzed, with synonyms)                 │  │
│  │  - Response.text (analyzed)                              │  │
│  │  - Tags (keyword)                                        │  │
│  │  - Metadata fields (selectively indexed)                 │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Vector Indexes (Pinecone):                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  - HNSW (Hierarchical Navigable Small World)             │  │
│  │  - M=16, efConstruction=200, efSearch=100                │  │
│  │  - Metadata filtering support                            │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Time-Series Indexes (InfluxDB):                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  - Timestamp (primary)                                   │  │
│  │  - Tags: tenantId, modelId, userId (for grouping)        │  │
│  │  - Fields: tokenCount, latency, cost (for aggregation)   │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. BATCH PROCESSING FLOWS

### 4.1 Nightly Batch Jobs

```
┌─────────────────────────────────────────────────────────────────┐
│                 Batch Processing Schedule                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Job 1: Similarity Graph Builder                                │
│  Schedule: Daily at 2:00 AM UTC                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  1. Extract all prompts from last 30 days                │  │
│  │  2. Compute pairwise cosine similarity (batch)           │  │
│  │  3. Filter: Keep only pairs with similarity > 0.85       │  │
│  │  4. Create SIMILAR_TO edges in graph                     │  │
│  │  5. Prune old edges (> 90 days)                          │  │
│  │  Duration: ~2 hours for 10M prompts                      │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Job 2: Prompt Template Extractor                               │
│  Schedule: Daily at 3:00 AM UTC                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  1. Identify prompt clusters (DBSCAN on embeddings)      │  │
│  │  2. Extract common patterns within clusters              │  │
│  │  3. Generate template strings with {{placeholders}}      │  │
│  │  4. Link prompts to templates (REUSED edges)             │  │
│  │  5. Store templates as separate nodes                    │  │
│  │  Duration: ~1 hour                                       │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Job 3: Anomaly Detection                                       │
│  Schedule: Daily at 4:00 AM UTC                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  1. Load time-series metrics (latency, cost, tokens)     │  │
│  │  2. Train/update anomaly detection model (Isolation Fst) │  │
│  │  3. Flag anomalous prompts/responses                     │  │
│  │  4. Generate alerts for investigation                    │  │
│  │  5. Update anomaly tags in graph                         │  │
│  │  Duration: ~30 minutes                                   │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Job 4: Aggregation Rollups                                     │
│  Schedule: Hourly                                               │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  1. Aggregate metrics by tenant, model, user             │  │
│  │  2. Compute hourly/daily/monthly summaries               │  │
│  │  3. Update materialized views                            │  │
│  │  4. Store rollups in time-series DB                      │  │
│  │  Duration: ~5 minutes                                    │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Job 5: Data Archival                                           │
│  Schedule: Weekly (Sunday at 1:00 AM UTC)                       │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  1. Identify data > 180 days old                         │  │
│  │  2. Export to Parquet format                             │  │
│  │  3. Upload to S3 Glacier                                 │  │
│  │  4. Delete from hot storage                              │  │
│  │  5. Update catalog with archive metadata                 │  │
│  │  Duration: ~4 hours                                      │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Job 6: Compliance Reports                                      │
│  Schedule: Monthly (1st of month)                               │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  1. Generate audit reports                               │  │
│  │  2. PII usage statistics                                 │  │
│  │  3. Data retention compliance check                      │  │
│  │  4. Export to PDF and send to stakeholders               │  │
│  │  Duration: ~1 hour                                       │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. INTEGRATION PROTOCOLS

### 5.1 LLM-Observatory Integration

**Event Stream Configuration:**
```yaml
kafka:
  topics:
    - name: llm.prompts.v1
      partitions: 12
      replication: 3
      retention: 7d
      compression: snappy

    - name: llm.responses.v1
      partitions: 12
      replication: 3
      retention: 7d
      compression: snappy

    - name: llm.sessions.v1
      partitions: 4
      replication: 3
      retention: 30d
      compression: snappy

consumer_groups:
  - name: memory-graph-ingestion
    topics:
      - llm.prompts.v1
      - llm.responses.v1
      - llm.sessions.v1
    config:
      auto.offset.reset: earliest
      enable.auto.commit: false
      max.poll.records: 500
      max.poll.interval.ms: 300000
      session.timeout.ms: 30000
```

**Producer Configuration (Observatory):**
```python
from kafka import KafkaProducer
import json

producer = KafkaProducer(
    bootstrap_servers=['kafka-1:9092', 'kafka-2:9092', 'kafka-3:9092'],
    value_serializer=lambda v: json.dumps(v).encode('utf-8'),
    compression_type='snappy',
    acks='all',  # Wait for all replicas
    retries=3,
    max_in_flight_requests_per_connection=5,
    enable_idempotence=True  # Exactly-once semantics
)

def send_prompt_event(prompt_data):
    producer.send(
        'llm.prompts.v1',
        key=prompt_data['session']['id'].encode('utf-8'),  # Partition by session
        value=prompt_data,
        headers=[
            ('schema_version', b'1.0'),
            ('source', b'observatory')
        ]
    )
    producer.flush()
```

### 5.2 LLM-Registry Integration

**gRPC Service Definition:**
```protobuf
syntax = "proto3";

package llm.registry.v1;

service RegistryService {
  rpc GetModel(GetModelRequest) returns (GetModelResponse);
  rpc BatchGetModels(BatchGetModelsRequest) returns (BatchGetModelsResponse);
  rpc StreamModelUpdates(StreamModelUpdatesRequest) returns (stream ModelUpdate);
}

message GetModelRequest {
  string model_id = 1;
}

message GetModelResponse {
  Model model = 1;
}

message Model {
  string id = 1;
  string provider = 2;
  string family = 3;
  string version = 4;
  int32 context_window = 5;
  int32 max_output_tokens = 6;
  double input_cost_per_1k = 7;
  double output_cost_per_1k = 8;
  repeated string capabilities = 9;
  google.protobuf.Timestamp released_at = 10;
  google.protobuf.Timestamp deprecated_at = 11;
  map<string, string> metadata = 12;
}

message BatchGetModelsRequest {
  repeated string model_ids = 1;
}

message BatchGetModelsResponse {
  map<string, Model> models = 1;
}

message StreamModelUpdatesRequest {
  // Empty - stream all updates
}

message ModelUpdate {
  enum UpdateType {
    CREATED = 0;
    UPDATED = 1;
    DEPRECATED = 2;
  }

  UpdateType type = 1;
  Model model = 2;
  google.protobuf.Timestamp timestamp = 3;
}
```

**Client Implementation (Memory-Graph):**
```go
package registry

import (
    "context"
    "time"

    "google.golang.org/grpc"
    pb "github.com/llm-devops/registry/proto"
)

type RegistryClient struct {
    client pb.RegistryServiceClient
    cache  *ModelCache
}

func NewRegistryClient(addr string) (*RegistryClient, error) {
    conn, err := grpc.Dial(addr, grpc.WithInsecure())
    if err != nil {
        return nil, err
    }

    client := pb.NewRegistryServiceClient(conn)
    cache := NewModelCache(5 * time.Minute) // 5min TTL

    return &RegistryClient{
        client: client,
        cache:  cache,
    }, nil
}

func (c *RegistryClient) GetModel(ctx context.Context, modelID string) (*pb.Model, error) {
    // Check cache first
    if model := c.cache.Get(modelID); model != nil {
        return model, nil
    }

    // Cache miss - fetch from registry
    resp, err := c.client.GetModel(ctx, &pb.GetModelRequest{
        ModelId: modelID,
    })
    if err != nil {
        return nil, err
    }

    // Update cache
    c.cache.Set(modelID, resp.Model)

    return resp.Model, nil
}

func (c *RegistryClient) StartModelUpdateStream(ctx context.Context) error {
    stream, err := c.client.StreamModelUpdates(ctx, &pb.StreamModelUpdatesRequest{})
    if err != nil {
        return err
    }

    go func() {
        for {
            update, err := stream.Recv()
            if err != nil {
                // Handle error and reconnect
                return
            }

            // Update cache
            c.cache.Set(update.Model.Id, update.Model)
        }
    }()

    return nil
}
```

### 5.3 LLM-Data-Vault Integration

**REST API Specification:**
```yaml
openapi: 3.0.0
info:
  title: LLM Data Vault API
  version: 1.0.0

paths:
  /v1/secrets:
    post:
      summary: Store encrypted data
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                data:
                  type: string
                  description: Base64-encoded data to encrypt
                tenantId:
                  type: string
                metadata:
                  type: object
      responses:
        '201':
          description: Secret created
          content:
            application/json:
              schema:
                type: object
                properties:
                  secretId:
                    type: string
                  version:
                    type: integer

  /v1/secrets/{secretId}:
    get:
      summary: Retrieve decrypted data
      parameters:
        - name: secretId
          in: path
          required: true
          schema:
            type: string
        - name: version
          in: query
          schema:
            type: integer
      responses:
        '200':
          description: Secret retrieved
          content:
            application/json:
              schema:
                type: object
                properties:
                  data:
                    type: string
                  metadata:
                    type: object
                  createdAt:
                    type: string
                    format: date-time
```

**Client Implementation:**
```typescript
import axios, { AxiosInstance } from 'axios';
import * as crypto from 'crypto';

export class DataVaultClient {
  private client: AxiosInstance;

  constructor(baseURL: string, apiKey: string) {
    this.client = axios.create({
      baseURL,
      headers: {
        'Authorization': `Bearer ${apiKey}`,
        'Content-Type': 'application/json'
      },
      httpsAgent: new https.Agent({
        // Mutual TLS
        cert: fs.readFileSync('client-cert.pem'),
        key: fs.readFileSync('client-key.pem'),
        ca: fs.readFileSync('ca-cert.pem')
      })
    });
  }

  async storeSecret(data: string, tenantId: string, metadata?: object): Promise<string> {
    // Base64 encode data
    const encoded = Buffer.from(data).toString('base64');

    const response = await this.client.post('/v1/secrets', {
      data: encoded,
      tenantId,
      metadata
    });

    return response.data.secretId;
  }

  async retrieveSecret(secretId: string, version?: number): Promise<string> {
    const response = await this.client.get(`/v1/secrets/${secretId}`, {
      params: { version }
    });

    // Base64 decode
    return Buffer.from(response.data.data, 'base64').toString('utf-8');
  }
}

// Usage in Memory-Graph
async function storePIIPrompt(promptText: string, tenantId: string) {
  const vaultClient = new DataVaultClient(
    process.env.VAULT_URL,
    process.env.VAULT_API_KEY
  );

  // Detect PII
  const hasPII = detectPII(promptText);

  if (hasPII) {
    // Store full text in vault
    const secretId = await vaultClient.storeSecret(promptText, tenantId, {
      type: 'prompt',
      pii: true
    });

    // Store only reference in graph
    return {
      text: '[ENCRYPTED - See Vault]',
      textSecretId: secretId,
      hasPII: true
    };
  } else {
    // Store directly in graph
    return {
      text: promptText,
      hasPII: false
    };
  }
}
```

---

## 6. PERFORMANCE TUNING

### 6.1 Throughput Optimization

**Write Path Optimization:**
```javascript
// Batch writes for better throughput
class BatchWriter {
  private batch: any[] = [];
  private readonly batchSize = 1000;
  private readonly flushInterval = 100; // ms

  constructor(private graphDB: Neo4jDriver) {
    // Periodic flush
    setInterval(() => this.flush(), this.flushInterval);
  }

  async write(node: Node, edges: Edge[]) {
    this.batch.push({ node, edges });

    if (this.batch.length >= this.batchSize) {
      await this.flush();
    }
  }

  private async flush() {
    if (this.batch.length === 0) return;

    const session = this.graphDB.session();

    try {
      // Single transaction for entire batch
      await session.writeTransaction(async tx => {
        // Use UNWIND for bulk insert
        const query = `
          UNWIND $batch AS item
          MERGE (n:Prompt {id: item.node.id})
          SET n += item.node.properties

          WITH n, item
          UNWIND item.edges AS edge
          MATCH (target {id: edge.targetId})
          MERGE (n)-[r:${edge.type}]->(target)
          SET r += edge.properties
        `;

        await tx.run(query, { batch: this.batch });
      });

      this.batch = [];
    } finally {
      await session.close();
    }
  }
}
```

### 6.2 Query Optimization

**Connection Pooling:**
```python
from neo4j import GraphDatabase
import redis
from pinecone import Pinecone

class OptimizedConnectionManager:
    def __init__(self):
        # Neo4j connection pool
        self.neo4j = GraphDatabase.driver(
            "bolt://neo4j:7687",
            auth=("neo4j", "password"),
            max_connection_pool_size=50,
            connection_acquisition_timeout=60.0
        )

        # Redis connection pool
        self.redis = redis.ConnectionPool(
            host='redis',
            port=6379,
            db=0,
            max_connections=100
        )

        # Pinecone (already pooled internally)
        self.pinecone = Pinecone(api_key="...")

    def get_neo4j_session(self):
        return self.neo4j.session(database="graph")

    def get_redis_client(self):
        return redis.Redis(connection_pool=self.redis)
```

---

## 7. MONITORING DATA FLOWS

**Key Metrics to Track:**

```yaml
metrics:
  ingestion:
    - name: events_received_total
      type: counter
      labels: [topic, status]
      description: Total events received from Kafka

    - name: enrichment_duration_seconds
      type: histogram
      labels: [stage]
      buckets: [0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
      description: Time spent in enrichment stages

    - name: write_latency_seconds
      type: histogram
      labels: [storage_backend]
      buckets: [0.01, 0.05, 0.1, 0.5, 1.0]
      description: Write latency per storage backend

  query:
    - name: queries_total
      type: counter
      labels: [type, status]
      description: Total queries executed

    - name: query_duration_seconds
      type: histogram
      labels: [type]
      buckets: [0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
      description: Query execution time

    - name: cache_hit_rate
      type: gauge
      labels: [layer]
      description: Cache hit rate percentage

  storage:
    - name: graph_nodes_total
      type: gauge
      description: Total nodes in graph

    - name: graph_edges_total
      type: gauge
      description: Total edges in graph

    - name: storage_size_bytes
      type: gauge
      labels: [backend]
      description: Storage size per backend
```

This comprehensive data flow specification provides all the details needed for implementing the LLM-Memory-Graph system's data pipelines, integrations, and optimizations.
