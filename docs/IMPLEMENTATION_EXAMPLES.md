# LLM-Memory-Graph Implementation Examples

## Table of Contents
1. [Client SDK Usage](#client-sdk-usage)
2. [API Integration Examples](#api-integration-examples)
3. [Query Examples](#query-examples)
4. [Advanced Use Cases](#advanced-use-cases)
5. [Custom Integrations](#custom-integrations)

---

## 1. CLIENT SDK USAGE

### 1.1 Python Client

```python
from llm_memory_graph import MemoryGraphClient
from datetime import datetime, timedelta

# Initialize client
client = MemoryGraphClient(
    endpoint="https://api.memory-graph.example.com",
    api_key="sk-...",
    timeout=30
)

# Record a prompt
prompt_id = client.record_prompt(
    text="Explain quantum computing in simple terms",
    session_id="session_123",
    user_id="user_456",
    model_id="gpt-4-0125-preview",
    parameters={
        "temperature": 0.7,
        "max_tokens": 500
    },
    tags=["physics", "education"],
    metadata={
        "application": "chatbot-v2",
        "source": "web"
    }
)

print(f"Prompt ID: {prompt_id}")

# Record response
response_id = client.record_response(
    prompt_id=prompt_id,
    text="Quantum computing leverages quantum mechanics...",
    finish_reason="stop",
    usage={
        "prompt_tokens": 15,
        "completion_tokens": 250,
        "total_tokens": 265
    },
    metrics={
        "latency": 1200,  # milliseconds
        "ttft": 100,      # time to first token
        "throughput": 208.33  # tokens/sec
    }
)

print(f"Response ID: {response_id}")

# Query lineage
lineage = client.get_lineage(
    node_id=prompt_id,
    direction="downstream",
    depth=3
)

print("Lineage:")
for node in lineage.descendants:
    print(f"  - {node.type}: {node.id} (depth: {node.depth})")

# Semantic search
similar_prompts = client.semantic_search(
    query="quantum mechanics basics",
    filters={
        "user_id": "user_456",
        "date_range": {
            "start": datetime.now() - timedelta(days=30),
            "end": datetime.now()
        }
    },
    limit=10,
    threshold=0.85
)

print("\nSimilar prompts:")
for result in similar_prompts:
    print(f"  - {result.text[:50]}... (similarity: {result.similarity:.2f})")

# Get session graph
session_graph = client.get_session_graph(
    session_id="session_123",
    format="cytoscape"
)

# Export for visualization
with open("session_graph.json", "w") as f:
    json.dump(session_graph, f, indent=2)
```

### 1.2 Node.js Client

```javascript
import { MemoryGraphClient } from '@llm-devops/memory-graph-client';

// Initialize client
const client = new MemoryGraphClient({
  endpoint: 'https://api.memory-graph.example.com',
  apiKey: process.env.MEMORY_GRAPH_API_KEY,
  timeout: 30000
});

// Record prompt with async/await
async function recordInteraction() {
  try {
    // Record prompt
    const { id: promptId } = await client.recordPrompt({
      text: 'What are the benefits of TypeScript?',
      sessionId: 'session_789',
      userId: 'user_123',
      modelId: 'gpt-4',
      parameters: {
        temperature: 0.7,
        maxTokens: 500
      },
      tags: ['programming', 'typescript']
    });

    console.log(`Prompt ID: ${promptId}`);

    // Simulate LLM response
    const responseText = 'TypeScript offers several benefits...';

    // Record response
    const { id: responseId } = await client.recordResponse({
      promptId,
      text: responseText,
      finishReason: 'stop',
      usage: {
        promptTokens: 12,
        completionTokens: 150,
        totalTokens: 162
      },
      metrics: {
        latency: 800,
        ttft: 80,
        throughput: 187.5,
        cost: 0.00486
      }
    });

    console.log(`Response ID: ${responseId}`);

    return { promptId, responseId };
  } catch (error) {
    console.error('Error recording interaction:', error);
    throw error;
  }
}

// Query with GraphQL
async function queryWithGraphQL() {
  const query = `
    query GetPromptLineage($promptId: ID!, $depth: Int!) {
      prompt(id: $promptId) {
        id
        text
        createdAt
        session {
          id
          userId
          startedAt
        }
        responses {
          id
          text
          metrics {
            latency
            cost
          }
          derivatives(depth: $depth) {
            prompt {
              id
              text
            }
            response {
              id
              text
            }
          }
        }
      }
    }
  `;

  const result = await client.graphql(query, {
    promptId: 'prompt_123',
    depth: 3
  });

  console.log('Query result:', JSON.stringify(result, null, 2));
  return result;
}

// Stream real-time updates
async function streamUpdates() {
  const stream = client.streamUpdates({
    filters: {
      userId: 'user_123',
      modelId: 'gpt-4'
    }
  });

  console.log('Streaming updates...');

  for await (const event of stream) {
    console.log(`Event: ${event.type}`, event.data);

    if (event.type === 'prompt') {
      console.log(`New prompt: ${event.data.text}`);
    } else if (event.type === 'response') {
      console.log(`New response: ${event.data.text}`);
    }
  }
}

// Export session data
async function exportSession(sessionId, format = 'json') {
  const data = await client.exportSession(sessionId, { format });

  // Save to file
  const fs = require('fs');
  const filename = `session_${sessionId}.${format}`;
  fs.writeFileSync(filename, data);

  console.log(`Session exported to ${filename}`);
}

// Run examples
(async () => {
  await recordInteraction();
  await queryWithGraphQL();
  await exportSession('session_789', 'graphml');
})();
```

### 1.3 Go Client

```go
package main

import (
    "context"
    "fmt"
    "time"

    memorygraph "github.com/llm-devops/memory-graph-go"
)

func main() {
    // Initialize client
    client := memorygraph.NewClient(memorygraph.Config{
        Endpoint: "https://api.memory-graph.example.com",
        APIKey:   os.Getenv("MEMORY_GRAPH_API_KEY"),
        Timeout:  30 * time.Second,
    })

    ctx := context.Background()

    // Record prompt
    promptReq := &memorygraph.RecordPromptRequest{
        Text:      "Explain goroutines in Go",
        SessionID: "session_456",
        UserID:    "user_789",
        ModelID:   "gpt-4",
        Parameters: map[string]interface{}{
            "temperature": 0.7,
            "maxTokens":   500,
        },
        Tags: []string{"golang", "concurrency"},
    }

    promptResp, err := client.RecordPrompt(ctx, promptReq)
    if err != nil {
        panic(err)
    }

    fmt.Printf("Prompt ID: %s\n", promptResp.ID)

    // Record response
    responseReq := &memorygraph.RecordResponseRequest{
        PromptID:     promptResp.ID,
        Text:         "Goroutines are lightweight threads...",
        FinishReason: "stop",
        Usage: memorygraph.Usage{
            PromptTokens:     10,
            CompletionTokens: 200,
            TotalTokens:      210,
        },
        Metrics: memorygraph.Metrics{
            Latency:    1000,
            TTFT:       90,
            Throughput: 200.0,
            Cost:       0.0063,
        },
    }

    responseResp, err := client.RecordResponse(ctx, responseReq)
    if err != nil {
        panic(err)
    }

    fmt.Printf("Response ID: %s\n", responseResp.ID)

    // Query lineage
    lineageReq := &memorygraph.GetLineageRequest{
        NodeID:    promptResp.ID,
        Direction: memorygraph.DirectionDownstream,
        Depth:     3,
    }

    lineage, err := client.GetLineage(ctx, lineageReq)
    if err != nil {
        panic(err)
    }

    fmt.Println("Lineage:")
    for _, node := range lineage.Descendants {
        fmt.Printf("  - %s: %s (depth: %d)\n", node.Type, node.ID, node.Depth)
    }

    // Semantic search with retry
    searchReq := &memorygraph.SemanticSearchRequest{
        Query: "Go concurrency patterns",
        Filters: memorygraph.SearchFilters{
            UserID: "user_789",
            DateRange: memorygraph.DateRange{
                Start: time.Now().Add(-30 * 24 * time.Hour),
                End:   time.Now(),
            },
        },
        Limit:     10,
        Threshold: 0.85,
    }

    results, err := client.SemanticSearch(ctx, searchReq)
    if err != nil {
        panic(err)
    }

    fmt.Println("\nSimilar prompts:")
    for _, result := range results {
        fmt.Printf("  - %s... (similarity: %.2f)\n",
            result.Text[:50], result.Similarity)
    }
}

// Example with streaming
func streamExample(client *memorygraph.Client) {
    ctx := context.Background()

    streamReq := &memorygraph.StreamUpdatesRequest{
        Filters: memorygraph.StreamFilters{
            UserID:  "user_789",
            ModelID: "gpt-4",
        },
    }

    stream, err := client.StreamUpdates(ctx, streamReq)
    if err != nil {
        panic(err)
    }

    fmt.Println("Streaming updates...")

    for {
        event, err := stream.Recv()
        if err == io.EOF {
            break
        }
        if err != nil {
            panic(err)
        }

        fmt.Printf("Event: %s\n", event.Type)

        switch event.Type {
        case "prompt":
            fmt.Printf("New prompt: %s\n", event.Prompt.Text)
        case "response":
            fmt.Printf("New response: %s\n", event.Response.Text)
        }
    }
}
```

---

## 2. API INTEGRATION EXAMPLES

### 2.1 REST API Examples

**Create Session:**
```bash
curl -X POST https://api.memory-graph.example.com/api/v1/sessions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer sk-..." \
  -d '{
    "userId": "user_123",
    "applicationId": "chatbot-v2",
    "purpose": "chat",
    "metadata": {
      "ip": "192.168.1.1",
      "userAgent": "Mozilla/5.0..."
    }
  }'

# Response:
{
  "id": "session_abc123",
  "userId": "user_123",
  "startedAt": "2025-11-06T10:00:00Z",
  "status": "active"
}
```

**Batch Ingest:**
```bash
curl -X POST https://api.memory-graph.example.com/api/v1/batch/prompts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer sk-..." \
  -d '{
    "prompts": [
      {
        "text": "What is AI?",
        "sessionId": "session_abc123",
        "userId": "user_123",
        "modelId": "gpt-4"
      },
      {
        "text": "Explain machine learning",
        "sessionId": "session_abc123",
        "userId": "user_123",
        "modelId": "gpt-4"
      }
    ]
  }'

# Response:
{
  "inserted": 2,
  "ids": ["prompt_001", "prompt_002"],
  "errors": []
}
```

**Complex Query:**
```bash
curl -X POST https://api.memory-graph.example.com/api/v1/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer sk-..." \
  -d '{
    "query": {
      "type": "graph_traversal",
      "start_node": "prompt_123",
      "traversal": [
        {"type": "GENERATED", "direction": "outgoing"},
        {"type": "DERIVED", "direction": "outgoing"}
      ],
      "filters": {
        "max_depth": 5,
        "node_types": ["PROMPT", "RESPONSE"]
      },
      "return": {
        "nodes": true,
        "edges": true,
        "properties": ["id", "text", "createdAt"]
      }
    }
  }'
```

### 2.2 GraphQL Examples

**Full Query Example:**
```graphql
query CompleteAnalysis($userId: String!, $startDate: DateTime!) {
  # Get user's sessions
  sessions(
    filters: {
      userId: $userId
      startedAt: { gte: $startDate }
    }
    pagination: { limit: 10, offset: 0 }
  ) {
    edges {
      node {
        id
        startedAt
        endedAt
        duration
        interactionCount
        totalTokens
        totalCost

        # Get prompts in this session
        prompts {
          id
          text
          tokenCount
          createdAt

          # Get responses
          responses {
            id
            text
            tokenCount
            finishReason
            metrics {
              latency
              ttft
              throughput
              cost
            }

            # Get derivative prompts
            derivatives {
              id
              text
              createdAt
            }
          }

          # Find similar prompts
          similarTo(threshold: 0.9) {
            prompt {
              id
              text
            }
            similarity
          }
        }

        # Get model usage stats
        models {
          id
          provider
          family
        }
      }
      cursor
    }
    pageInfo {
      hasNextPage
      hasPreviousPage
      totalCount
    }
  }

  # Get usage statistics
  statistics: userStatistics(
    userId: $userId
    period: { start: $startDate, end: null }
  ) {
    totalPrompts
    totalResponses
    totalTokens
    totalCost
    avgLatency
    avgThroughput
    modelBreakdown {
      modelId
      count
      tokens
      cost
    }
  }
}
```

**Mutation Example:**
```graphql
mutation RecordInteraction($input: InteractionInput!) {
  recordInteraction(input: $input) {
    prompt {
      id
      text
      createdAt
    }
    response {
      id
      text
      createdAt
      metrics {
        latency
        cost
      }
    }
    session {
      id
      interactionCount
    }
  }
}

# Variables:
{
  "input": {
    "prompt": {
      "text": "Explain neural networks",
      "sessionId": "session_123",
      "userId": "user_456",
      "modelId": "gpt-4",
      "parameters": {
        "temperature": 0.7
      }
    },
    "response": {
      "text": "Neural networks are...",
      "usage": {
        "promptTokens": 10,
        "completionTokens": 200
      },
      "metrics": {
        "latency": 1000
      }
    }
  }
}
```

### 2.3 WebSocket Streaming

```javascript
const WebSocket = require('ws');

const ws = new WebSocket('wss://api.memory-graph.example.com/v1/stream', {
  headers: {
    'Authorization': 'Bearer sk-...'
  }
});

ws.on('open', () => {
  console.log('Connected to stream');

  // Subscribe to specific filters
  ws.send(JSON.stringify({
    action: 'subscribe',
    filters: {
      userId: 'user_123',
      modelId: 'gpt-4',
      eventTypes: ['prompt', 'response']
    }
  }));
});

ws.on('message', (data) => {
  const event = JSON.parse(data);

  console.log('Event received:', event.type);

  switch (event.type) {
    case 'prompt':
      console.log('New prompt:', event.data.text);
      break;
    case 'response':
      console.log('New response:', event.data.text);
      console.log('Latency:', event.data.metrics.latency);
      break;
    case 'session_start':
      console.log('Session started:', event.data.sessionId);
      break;
    case 'session_end':
      console.log('Session ended:', event.data.sessionId);
      break;
  }
});

ws.on('error', (error) => {
  console.error('WebSocket error:', error);
});

ws.on('close', () => {
  console.log('Disconnected from stream');
});
```

---

## 3. QUERY EXAMPLES

### 3.1 Cypher Queries

**Find Most Reused Prompts:**
```cypher
// Find prompts that have been reused more than 10 times
MATCH (p:Prompt)<-[r:REUSED]-(reuser:Prompt)
WITH p, COUNT(r) AS reuseCount
WHERE reuseCount > 10
RETURN p.id, p.text, reuseCount
ORDER BY reuseCount DESC
LIMIT 20
```

**Analyze Prompt-Response Chains:**
```cypher
// Find chains where prompts led to errors
MATCH path = (p:Prompt)-[:GENERATED]->(r:Response {success: false})
              -[:DERIVED]->(p2:Prompt)-[:GENERATED]->(r2:Response)
WHERE r2.success = true
RETURN p.text AS originalPrompt,
       r.error AS error,
       p2.text AS followupPrompt,
       r2.text AS successfulResponse,
       r2.metrics.latency AS latency
LIMIT 10
```

**Session Analysis:**
```cypher
// Analyze a session's interaction flow
MATCH (s:Session {id: 'session_123'})<-[:PART_OF]-(p:Prompt)
      -[:GENERATED]->(r:Response)
OPTIONAL MATCH (r)-[:DERIVED]->(nextP:Prompt)
RETURN p.text AS prompt,
       r.text AS response,
       r.metrics.latency AS latency,
       COLLECT(nextP.text) AS followups
ORDER BY p.createdAt
```

**Cost Analysis:**
```cypher
// Calculate cost per user per model
MATCH (u:User)-[:INITIATED_BY]-(s:Session)-[:EXECUTED_BY]->(m:Model)
MATCH (s)<-[:PART_OF]-(r:Response)
WITH u, m, SUM(r.metrics.cost) AS totalCost, COUNT(r) AS responseCount
RETURN u.id AS userId,
       m.id AS modelId,
       totalCost,
       responseCount,
       totalCost / responseCount AS avgCostPerResponse
ORDER BY totalCost DESC
```

**Prompt Template Extraction:**
```cypher
// Find similar prompts and extract template
MATCH (p1:Prompt)-[s:SIMILAR_TO]->(p2:Prompt)
WHERE s.similarity > 0.95
WITH p1, COLLECT(p2.text) AS similarTexts
RETURN p1.id,
       p1.text,
       similarTexts,
       SIZE(similarTexts) AS clusterSize
ORDER BY clusterSize DESC
LIMIT 10
```

### 3.2 Advanced Graph Queries

**Multi-Hop Lineage:**
```cypher
// Find all descendants of a prompt within 5 hops
MATCH path = (root:Prompt {id: 'prompt_123'})-[:GENERATED|DERIVED*1..5]->(descendant)
RETURN root.text AS rootPrompt,
       [node IN NODES(path) | node.text] AS pathTexts,
       LENGTH(path) AS depth,
       descendant.text AS descendantText
ORDER BY depth, descendant.createdAt
```

**Influence Analysis:**
```cypher
// Find prompts that influenced the most responses
MATCH (p:Prompt)-[:GENERATED]->(r:Response)
      -[:DERIVED]->(dp:Prompt)-[:GENERATED]->(dr:Response)
WITH p, COUNT(DISTINCT dr) AS influencedResponses
WHERE influencedResponses > 5
RETURN p.id,
       p.text,
       influencedResponses,
       p.createdAt
ORDER BY influencedResponses DESC
LIMIT 20
```

**Circular Reference Detection:**
```cypher
// Detect potential circular dependencies
MATCH path = (p:Prompt)-[:DERIVED*]->(p)
RETURN p.id,
       p.text,
       LENGTH(path) AS cycleLength,
       [node IN NODES(path) | node.id] AS cycleNodes
```

### 3.3 Analytics Queries

**Time Series Analysis:**
```sql
-- InfluxDB query for latency trends
SELECT
  MEAN("latency") AS avg_latency,
  PERCENTILE("latency", 95) AS p95_latency,
  PERCENTILE("latency", 99) AS p99_latency,
  COUNT("latency") AS request_count
FROM "response_events"
WHERE
  time >= now() - 24h
  AND "modelId" = 'gpt-4'
GROUP BY time(1h), "tenantId"
```

**Aggregation Query:**
```sql
-- Daily usage summary
SELECT
  "tenantId",
  "modelId",
  SUM("tokenCount") AS total_tokens,
  SUM("cost") AS total_cost,
  COUNT(*) AS request_count,
  AVG("latency") AS avg_latency
FROM "response_events"
WHERE
  time >= now() - 7d
GROUP BY time(1d), "tenantId", "modelId"
ORDER BY total_cost DESC
```

---

## 4. ADVANCED USE CASES

### 4.1 Prompt Optimization Pipeline

```python
from llm_memory_graph import MemoryGraphClient
from sklearn.cluster import DBSCAN
import numpy as np

client = MemoryGraphClient(endpoint="...", api_key="...")

def optimize_prompts_for_user(user_id, days=30):
    """
    Analyze user's prompts and suggest optimizations
    """
    # Get user's prompts
    prompts = client.query_prompts(
        filters={
            "user_id": user_id,
            "date_range": {"days": days}
        },
        include_responses=True
    )

    # Extract features
    features = []
    for prompt in prompts:
        features.append({
            "id": prompt.id,
            "text": prompt.text,
            "embedding": prompt.embedding,
            "avg_latency": np.mean([r.metrics.latency for r in prompt.responses]),
            "success_rate": sum(r.success for r in prompt.responses) / len(prompt.responses),
            "avg_cost": np.mean([r.metrics.cost for r in prompt.responses])
        })

    # Cluster similar prompts
    embeddings = np.array([f["embedding"] for f in features])
    clustering = DBSCAN(eps=0.1, min_samples=3).fit(embeddings)

    # Analyze clusters
    clusters = {}
    for idx, label in enumerate(clustering.labels_):
        if label not in clusters:
            clusters[label] = []
        clusters[label].append(features[idx])

    # Generate recommendations
    recommendations = []
    for label, cluster_prompts in clusters.items():
        if label == -1:  # Noise
            continue

        # Find best performing prompt in cluster
        best_prompt = max(cluster_prompts,
                         key=lambda p: p["success_rate"] - p["avg_latency"]/1000)

        # Recommend template
        template = extract_template(best_prompt["text"])

        recommendations.append({
            "cluster_size": len(cluster_prompts),
            "recommended_template": template,
            "avg_latency": np.mean([p["avg_latency"] for p in cluster_prompts]),
            "avg_cost": np.mean([p["avg_cost"] for p in cluster_prompts]),
            "success_rate": np.mean([p["success_rate"] for p in cluster_prompts])
        })

    return recommendations

def extract_template(text):
    """
    Simple template extraction (replace with more sophisticated logic)
    """
    import re

    # Replace numbers with {{number}}
    text = re.sub(r'\b\d+\b', '{{number}}', text)

    # Replace quoted strings with {{string}}
    text = re.sub(r'"[^"]*"', '{{string}}', text)

    # Replace common entities with placeholders
    text = re.sub(r'\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\b', '{{entity}}', text)

    return text

# Usage
recommendations = optimize_prompts_for_user("user_123", days=30)

print("Prompt Optimization Recommendations:")
for i, rec in enumerate(recommendations, 1):
    print(f"\nRecommendation {i}:")
    print(f"  Cluster size: {rec['cluster_size']}")
    print(f"  Template: {rec['recommended_template']}")
    print(f"  Avg latency: {rec['avg_latency']:.0f}ms")
    print(f"  Avg cost: ${rec['avg_cost']:.4f}")
    print(f"  Success rate: {rec['success_rate']:.2%}")
```

### 4.2 Model Performance Comparison

```javascript
const { MemoryGraphClient } = require('@llm-devops/memory-graph-client');

const client = new MemoryGraphClient({
  endpoint: process.env.MEMORY_GRAPH_ENDPOINT,
  apiKey: process.env.MEMORY_GRAPH_API_KEY
});

async function compareModels(promptText, models) {
  const results = {};

  for (const model of models) {
    // Check if similar prompt was used with this model
    const similarPrompts = await client.semanticSearch({
      query: promptText,
      filters: {
        modelId: model,
        dateRange: { days: 90 }
      },
      limit: 10,
      threshold: 0.9
    });

    if (similarPrompts.length === 0) {
      results[model] = {
        available: false,
        message: 'No similar prompts found for this model'
      };
      continue;
    }

    // Aggregate metrics
    const metrics = similarPrompts.flatMap(p =>
      p.responses.map(r => r.metrics)
    );

    results[model] = {
      available: true,
      sampleSize: metrics.length,
      avgLatency: average(metrics.map(m => m.latency)),
      p95Latency: percentile(metrics.map(m => m.latency), 0.95),
      avgCost: average(metrics.map(m => m.cost)),
      successRate: metrics.filter(m => m.success).length / metrics.length,
      avgQuality: await assessQuality(similarPrompts)
    };
  }

  return results;
}

async function assessQuality(prompts) {
  // Simplified quality assessment
  // In production, use more sophisticated metrics
  const qualityScores = [];

  for (const prompt of prompts) {
    for (const response of prompt.responses) {
      // Check if there were follow-up corrections
      const hadCorrection = response.derivatives.length > 0;

      // Length appropriateness (not too short, not too long)
      const lengthScore = 1 - Math.abs(response.tokenCount - 200) / 200;

      // Completeness (finished naturally)
      const completenessScore = response.finishReason === 'stop' ? 1 : 0.5;

      const score = (lengthScore + completenessScore + (hadCorrection ? 0 : 1)) / 3;
      qualityScores.push(Math.max(0, Math.min(1, score)));
    }
  }

  return average(qualityScores);
}

function average(arr) {
  return arr.reduce((a, b) => a + b, 0) / arr.length;
}

function percentile(arr, p) {
  const sorted = arr.slice().sort((a, b) => a - b);
  const index = Math.ceil(sorted.length * p) - 1;
  return sorted[index];
}

// Usage
(async () => {
  const prompt = "Explain the difference between async and sync programming";
  const models = ['gpt-4', 'gpt-3.5-turbo', 'claude-3-opus', 'claude-3-sonnet'];

  const comparison = await compareModels(prompt, models);

  console.log('Model Performance Comparison:');
  console.log('===============================\n');

  for (const [model, stats] of Object.entries(comparison)) {
    console.log(`${model}:`);

    if (!stats.available) {
      console.log(`  ${stats.message}\n`);
      continue;
    }

    console.log(`  Sample size: ${stats.sampleSize}`);
    console.log(`  Avg latency: ${stats.avgLatency.toFixed(0)}ms`);
    console.log(`  P95 latency: ${stats.p95Latency.toFixed(0)}ms`);
    console.log(`  Avg cost: $${stats.avgCost.toFixed(4)}`);
    console.log(`  Success rate: ${(stats.successRate * 100).toFixed(1)}%`);
    console.log(`  Quality score: ${(stats.avgQuality * 100).toFixed(1)}%`);
    console.log('');
  }

  // Recommend best model
  const ranked = Object.entries(comparison)
    .filter(([_, stats]) => stats.available)
    .map(([model, stats]) => ({
      model,
      score: (
        (1 - stats.avgLatency / 5000) * 0.3 +    // Latency weight
        (1 - stats.avgCost / 0.1) * 0.2 +         // Cost weight
        stats.successRate * 0.2 +                 // Success weight
        stats.avgQuality * 0.3                    // Quality weight
      )
    }))
    .sort((a, b) => b.score - a.score);

  console.log('Recommended model:', ranked[0].model);
})();
```

### 4.3 Anomaly Detection

```python
from llm_memory_graph import MemoryGraphClient
from sklearn.ensemble import IsolationForest
import pandas as pd
import numpy as np

client = MemoryGraphClient(endpoint="...", api_key="...")

def detect_anomalies(tenant_id, lookback_hours=24):
    """
    Detect anomalous LLM interactions
    """
    # Fetch recent interactions
    interactions = client.query_interactions(
        filters={
            "tenant_id": tenant_id,
            "time_range": {"hours": lookback_hours}
        }
    )

    # Extract features
    features = []
    for interaction in interactions:
        features.append({
            "id": interaction.id,
            "prompt_length": len(interaction.prompt.text),
            "prompt_tokens": interaction.prompt.tokenCount,
            "response_length": len(interaction.response.text),
            "response_tokens": interaction.response.tokenCount,
            "latency": interaction.response.metrics.latency,
            "cost": interaction.response.metrics.cost,
            "ttft": interaction.response.metrics.ttft,
            "throughput": interaction.response.metrics.throughput,
            "hour_of_day": interaction.createdAt.hour,
            "has_error": not interaction.response.success
        })

    df = pd.DataFrame(features)

    # Prepare features for anomaly detection
    feature_cols = [
        "prompt_length", "prompt_tokens",
        "response_length", "response_tokens",
        "latency", "cost", "ttft", "throughput"
    ]

    X = df[feature_cols].fillna(0).values

    # Normalize features
    from sklearn.preprocessing import StandardScaler
    scaler = StandardScaler()
    X_scaled = scaler.fit_transform(X)

    # Train Isolation Forest
    clf = IsolationForest(
        contamination=0.1,  # Expect 10% anomalies
        random_state=42,
        n_estimators=100
    )

    predictions = clf.fit_predict(X_scaled)
    anomaly_scores = clf.score_samples(X_scaled)

    # Add predictions to dataframe
    df['is_anomaly'] = predictions == -1
    df['anomaly_score'] = anomaly_scores

    # Analyze anomalies
    anomalies = df[df['is_anomaly']].copy()
    anomalies['reasons'] = anomalies.apply(lambda row: analyze_anomaly(row, df), axis=1)

    return anomalies[['id', 'anomaly_score', 'reasons', 'latency', 'cost', 'has_error']]

def analyze_anomaly(row, df):
    """
    Determine why an interaction is anomalous
    """
    reasons = []

    # High latency
    if row['latency'] > df['latency'].quantile(0.95):
        reasons.append(f"High latency: {row['latency']:.0f}ms")

    # High cost
    if row['cost'] > df['cost'].quantile(0.95):
        reasons.append(f"High cost: ${row['cost']:.4f}")

    # Unusual token ratio
    token_ratio = row['response_tokens'] / max(row['prompt_tokens'], 1)
    avg_ratio = (df['response_tokens'] / df['prompt_tokens'].replace(0, 1)).mean()
    if abs(token_ratio - avg_ratio) > 2 * df['response_tokens'].std():
        reasons.append(f"Unusual token ratio: {token_ratio:.2f}")

    # Low throughput
    if row['throughput'] < df['throughput'].quantile(0.05):
        reasons.append(f"Low throughput: {row['throughput']:.1f} tokens/s")

    # Error
    if row['has_error']:
        reasons.append("Request failed")

    return "; ".join(reasons) if reasons else "General anomaly"

# Usage
anomalies = detect_anomalies("tenant_123", lookback_hours=24)

print(f"Detected {len(anomalies)} anomalies:")
print(anomalies.to_string(index=False))

# Alert on critical anomalies
critical = anomalies[anomalies['anomaly_score'] < -0.5]
if len(critical) > 0:
    print(f"\n⚠️  {len(critical)} CRITICAL anomalies detected!")
    for _, row in critical.iterrows():
        print(f"  - ID: {row['id']}")
        print(f"    Score: {row['anomaly_score']:.3f}")
        print(f"    Reasons: {row['reasons']}")
```

---

## 5. CUSTOM INTEGRATIONS

### 5.1 LangChain Custom Callback

```python
from langchain.callbacks.base import BaseCallbackHandler
from langchain.schema import LLMResult
from typing import Dict, Any, List
from llm_memory_graph import MemoryGraphClient
import uuid

class MemoryGraphCallbackHandler(BaseCallbackHandler):
    """Custom callback for LangChain integration"""

    def __init__(self,
                 endpoint: str,
                 api_key: str,
                 session_id: str = None,
                 user_id: str = None):
        self.client = MemoryGraphClient(endpoint=endpoint, api_key=api_key)
        self.session_id = session_id or str(uuid.uuid4())
        self.user_id = user_id or "anonymous"
        self.prompt_ids = {}

    def on_llm_start(
        self, serialized: Dict[str, Any], prompts: List[str], **kwargs: Any
    ) -> None:
        """Record prompts when LLM starts"""
        model_id = serialized.get("name", "unknown")

        for prompt in prompts:
            prompt_id = self.client.record_prompt(
                text=prompt,
                session_id=self.session_id,
                user_id=self.user_id,
                model_id=model_id,
                parameters=kwargs.get("invocation_params", {}),
                metadata={
                    "langchain_serialized": serialized,
                    "run_id": str(kwargs.get("run_id"))
                }
            )

            self.prompt_ids[prompt] = prompt_id

    def on_llm_end(self, response: LLMResult, **kwargs: Any) -> None:
        """Record responses when LLM ends"""
        for i, generations in enumerate(response.generations):
            for generation in generations:
                # Find corresponding prompt
                prompt = kwargs.get("prompts", [])[i] if i < len(kwargs.get("prompts", [])) else None
                prompt_id = self.prompt_ids.get(prompt)

                if not prompt_id:
                    continue

                # Extract metrics
                llm_output = response.llm_output or {}
                token_usage = llm_output.get("token_usage", {})

                self.client.record_response(
                    prompt_id=prompt_id,
                    text=generation.text,
                    finish_reason="stop",
                    usage={
                        "prompt_tokens": token_usage.get("prompt_tokens", 0),
                        "completion_tokens": token_usage.get("completion_tokens", 0),
                        "total_tokens": token_usage.get("total_tokens", 0)
                    },
                    metrics={
                        "latency": int((response.llm_output or {}).get("latency", 0) * 1000)
                    },
                    metadata={
                        "generation_info": generation.generation_info
                    }
                )

    def on_llm_error(
        self, error: BaseException, **kwargs: Any
    ) -> None:
        """Record errors"""
        print(f"LLM Error: {error}")
        # Could record error in memory graph here

# Usage with LangChain
from langchain.llms import OpenAI

callback = MemoryGraphCallbackHandler(
    endpoint="https://api.memory-graph.example.com",
    api_key="sk-...",
    session_id="langchain_session_123",
    user_id="user_456"
)

llm = OpenAI(temperature=0.7, callbacks=[callback])

response = llm.predict("What is the capital of France?")
print(response)
```

### 5.2 OpenTelemetry Integration

```go
package memorygraph

import (
    "context"
    "go.opentelemetry.io/otel"
    "go.opentelemetry.io/otel/attribute"
    "go.opentelemetry.io/otel/trace"
)

type TracingClient struct {
    client *Client
    tracer trace.Tracer
}

func NewTracingClient(config Config) *TracingClient {
    return &TracingClient{
        client: NewClient(config),
        tracer: otel.Tracer("llm-memory-graph"),
    }
}

func (tc *TracingClient) RecordPrompt(
    ctx context.Context,
    req *RecordPromptRequest,
) (*RecordPromptResponse, error) {
    ctx, span := tc.tracer.Start(ctx, "RecordPrompt")
    defer span.End()

    // Add attributes
    span.SetAttributes(
        attribute.String("llm.prompt.text", req.Text),
        attribute.String("llm.session.id", req.SessionID),
        attribute.String("llm.user.id", req.UserID),
        attribute.String("llm.model.id", req.ModelID),
        attribute.Int("llm.prompt.tokens", req.TokenCount),
    )

    // Call underlying client
    resp, err := tc.client.RecordPrompt(ctx, req)
    if err != nil {
        span.RecordError(err)
        return nil, err
    }

    span.SetAttributes(
        attribute.String("llm.prompt.id", resp.ID),
    )

    return resp, nil
}

func (tc *TracingClient) RecordResponse(
    ctx context.Context,
    req *RecordResponseRequest,
) (*RecordResponseResponse, error) {
    ctx, span := tc.tracer.Start(ctx, "RecordResponse")
    defer span.End()

    span.SetAttributes(
        attribute.String("llm.prompt.id", req.PromptID),
        attribute.Int("llm.response.tokens", req.Usage.CompletionTokens),
        attribute.Int("llm.response.latency", req.Metrics.Latency),
        attribute.Float64("llm.response.cost", req.Metrics.Cost),
        attribute.String("llm.response.finish_reason", req.FinishReason),
    )

    resp, err := tc.client.RecordResponse(ctx, req)
    if err != nil {
        span.RecordError(err)
        return nil, err
    }

    span.SetAttributes(
        attribute.String("llm.response.id", resp.ID),
    )

    return resp, nil
}
```

### 5.3 FastAPI Middleware

```python
from fastapi import FastAPI, Request, Response
from starlette.middleware.base import BaseHTTPMiddleware
from llm_memory_graph import MemoryGraphClient
import time
import json

class MemoryGraphMiddleware(BaseHTTPMiddleware):
    """Middleware to automatically track LLM API calls"""

    def __init__(self, app, endpoint: str, api_key: str):
        super().__init__(app)
        self.client = MemoryGraphClient(endpoint=endpoint, api_key=api_key)

    async def dispatch(self, request: Request, call_next):
        # Only track LLM endpoints
        if not request.url.path.startswith("/v1/chat/completions"):
            return await call_next(request)

        # Extract request data
        body = await request.body()
        request_data = json.loads(body) if body else {}

        # Extract prompt
        messages = request_data.get("messages", [])
        prompt_text = " ".join([msg.get("content", "") for msg in messages])

        # Record start time
        start_time = time.time()

        # Record prompt
        session_id = request.headers.get("X-Session-ID", "unknown")
        user_id = request.headers.get("X-User-ID", "unknown")
        model_id = request_data.get("model", "unknown")

        prompt_id = self.client.record_prompt(
            text=prompt_text,
            session_id=session_id,
            user_id=user_id,
            model_id=model_id,
            parameters={
                "temperature": request_data.get("temperature"),
                "max_tokens": request_data.get("max_tokens"),
                "top_p": request_data.get("top_p")
            },
            metadata={
                "endpoint": str(request.url),
                "method": request.method,
                "client_ip": request.client.host
            }
        )

        # Process request
        response = await call_next(request)

        # Calculate latency
        latency = int((time.time() - start_time) * 1000)

        # Parse response
        response_body = b""
        async for chunk in response.body_iterator:
            response_body += chunk

        response_data = json.loads(response_body) if response_body else {}

        # Record response
        choices = response_data.get("choices", [])
        response_text = " ".join([
            choice.get("message", {}).get("content", "")
            for choice in choices
        ])

        usage = response_data.get("usage", {})

        self.client.record_response(
            prompt_id=prompt_id,
            text=response_text,
            finish_reason=choices[0].get("finish_reason") if choices else "unknown",
            usage={
                "prompt_tokens": usage.get("prompt_tokens", 0),
                "completion_tokens": usage.get("completion_tokens", 0),
                "total_tokens": usage.get("total_tokens", 0)
            },
            metrics={
                "latency": latency,
                "cost": estimate_cost(model_id, usage)
            }
        )

        # Return response
        return Response(
            content=response_body,
            status_code=response.status_code,
            headers=dict(response.headers)
        )

def estimate_cost(model_id: str, usage: dict) -> float:
    """Estimate cost based on model and token usage"""
    pricing = {
        "gpt-4": {"input": 0.03, "output": 0.06},
        "gpt-3.5-turbo": {"input": 0.0015, "output": 0.002},
    }

    if model_id not in pricing:
        return 0.0

    input_cost = (usage.get("prompt_tokens", 0) / 1000) * pricing[model_id]["input"]
    output_cost = (usage.get("completion_tokens", 0) / 1000) * pricing[model_id]["output"]

    return input_cost + output_cost

# Usage
app = FastAPI()

app.add_middleware(
    MemoryGraphMiddleware,
    endpoint="https://api.memory-graph.example.com",
    api_key="sk-..."
)

@app.post("/v1/chat/completions")
async def chat_completion(request: Request):
    # Your LLM API logic here
    pass
```

---

This comprehensive implementation guide provides practical examples for integrating and using LLM-Memory-Graph across various programming languages, frameworks, and use cases.
