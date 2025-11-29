# Quick Start Guide

Get up and running with LLM Memory Graph in minutes.

## Table of Contents

- [What is LLM Memory Graph?](#what-is-llm-memory-graph)
- [Installation](#installation)
- [Starting the Server](#starting-the-server)
- [Your First Client](#your-first-client)
- [Basic Operations](#basic-operations)
- [Next Steps](#next-steps)

## What is LLM Memory Graph?

LLM Memory Graph is a graph-based memory system for tracking conversations, prompts, responses, and tool invocations in LLM applications. It provides:

- **Session Management** - Organize interactions into sessions
- **Prompt Lineage** - Track prompt-response relationships
- **Tool Invocation Tracking** - Monitor tool calls and results
- **Template Management** - Reusable prompt templates
- **Query Capabilities** - Powerful querying and filtering
- **Multiple Interfaces** - TypeScript, Rust, CLI, and gRPC

## Installation

### Server Installation

#### Using Cargo

```bash
# Install from source
cargo install --path crates/llm-memory-graph-cli

# Or clone and build
git clone https://github.com/globalbusinessadvisors/llm-memory-graph.git
cd llm-memory-graph
cargo build --release
```

#### Using Docker

```bash
docker pull ghcr.io/globalbusinessadvisors/llm-memory-graph:latest
```

### Client Installation

#### TypeScript/JavaScript

```bash
npm install @llm-dev-ops/llm-memory-graph-client
```

#### Rust

Add to `Cargo.toml`:

```toml
[dependencies]
llm-memory-graph-client = "0.1.0"
```

## Starting the Server

### Local Development

```bash
# Start server with default settings
llm-memory-graph server start

# Custom database path
llm-memory-graph --db-path ./my-data server start

# Custom port
llm-memory-graph server start --port 8080
```

The server will start on `localhost:50051` by default.

### Docker

```bash
# Run server in Docker
docker run -p 50051:50051 \
  -v $(pwd)/data:/data \
  ghcr.io/globalbusinessadvisors/llm-memory-graph:latest

# With custom configuration
docker run -p 50051:50051 \
  -v $(pwd)/data:/data \
  -v $(pwd)/config.toml:/config.toml \
  ghcr.io/globalbusinessadvisors/llm-memory-graph:latest \
  --config /config.toml
```

### Verify Server is Running

```bash
# Check server health
llm-memory-graph server health

# Or use CLI
llm-memory-graph stats
```

## Your First Client

### TypeScript Example

Create `example.ts`:

```typescript
import { MemoryGraphClient } from '@llm-dev-ops/llm-memory-graph-client';

async function main() {
  // 1. Create client
  const client = new MemoryGraphClient({
    address: 'localhost:50051',
    useTls: false
  });

  try {
    // 2. Create a session
    const session = await client.createSession({
      metadata: {
        user: 'alice',
        application: 'quickstart'
      }
    });

    console.log('Session created:', session.id);

    // 3. Add a prompt
    const prompt = await client.addPrompt({
      sessionId: session.id,
      content: 'What is 2+2?',
      metadata: {
        model: 'gpt-4',
        temperature: 0.7
      }
    });

    console.log('Prompt added:', prompt.id);

    // 4. Add a response
    const response = await client.addResponse({
      promptId: prompt.id,
      content: 'The answer is 4.',
      tokenUsage: {
        promptTokens: 8,
        completionTokens: 6,
        totalTokens: 14
      },
      metadata: {
        model: 'gpt-4',
        finishReason: 'stop'
      }
    });

    console.log('Response added:', response.id);

    // 5. Query session nodes
    const nodes = await client.getSessionNodes(session.id);
    console.log(`Session has ${nodes.length} nodes`);

    // 6. Get health status
    const health = await client.checkHealth();
    console.log('Server status:', health.status);

  } finally {
    // 7. Always close the client
    await client.close();
  }
}

main().catch(console.error);
```

Run it:

```bash
npx ts-node example.ts
```

### Rust Example

Create `src/main.rs`:

```rust
use llm_memory_graph_client::{Client, ClientConfig, TokenUsage};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create client
    let config = ClientConfig {
        endpoint: "http://localhost:50051".to_string(),
        ..Default::default()
    };

    let mut client = Client::new(config).await?;

    // 2. Create session
    let mut metadata = HashMap::new();
    metadata.insert("user".to_string(), "bob".to_string());
    metadata.insert("application".to_string(), "quickstart".to_string());

    let session = client.create_session(Some(metadata)).await?;
    println!("Session created: {}", session.id);

    // 3. Add prompt
    let mut prompt_meta = HashMap::new();
    prompt_meta.insert("model".to_string(), "gpt-4".to_string());

    let prompt = client.add_prompt(
        &session.id,
        "What is Rust?",
        Some(prompt_meta)
    ).await?;

    println!("Prompt added: {}", prompt.id);

    // 4. Add response
    let response = client.add_response(
        &prompt.id,
        "Rust is a systems programming language.",
        Some(TokenUsage {
            prompt_tokens: 7,
            completion_tokens: 8,
            total_tokens: 15,
        }),
        None
    ).await?;

    println!("Response added: {}", response.id);

    // 5. Query nodes
    let nodes = client.get_session_nodes(&session.id, None, None).await?;
    println!("Session has {} nodes", nodes.len());

    Ok(())
}
```

Run it:

```bash
cargo run
```

## Basic Operations

### Working with Sessions

Sessions organize related interactions.

```typescript
// Create session
const session = await client.createSession({
  metadata: {
    user: 'alice',
    context: 'customer_support'
  }
});

// Get session details
const retrieved = await client.getSession(session.id);

// Close session when done
await client.closeSession(session.id);
```

### Adding Prompts and Responses

Track conversation flow:

```typescript
// Add user prompt
const prompt = await client.addPrompt({
  sessionId: session.id,
  content: 'How do I reset my password?',
  metadata: {
    model: 'gpt-4',
    temperature: 0.5
  }
});

// Add AI response
const response = await client.addResponse({
  promptId: prompt.id,
  content: 'To reset your password, click...',
  tokenUsage: {
    promptTokens: 12,
    completionTokens: 45,
    totalTokens: 57
  },
  metadata: {
    model: 'gpt-4',
    finishReason: 'stop',
    latencyMs: 1200
  }
});
```

### Tracking Tool Invocations

Monitor tool usage:

```typescript
const toolInvocation = await client.addToolInvocation({
  toolInvocation: {
    responseId: response.id,
    toolName: 'search_database',
    parameters: {
      query: 'password reset procedure',
      limit: 5
    },
    status: 'success',
    result: { found: 3, items: [...] },
    durationMs: 245,
    timestamp: new Date()
  }
});
```

### Querying Data

Find nodes with filters:

```typescript
// Get all prompts in session
const prompts = await client.queryNodes({
  sessionId: session.id,
  nodeType: 'prompt'
});

// Get recent responses
const recentResponses = await client.queryNodes({
  nodeType: 'response',
  after: new Date('2024-01-01'),
  limit: 10
});

// Get session conversation
const nodes = await client.getSessionNodes(session.id);
```

### Using Templates

Create reusable prompts:

```typescript
// Create template
const template = await client.createTemplate({
  template: {
    name: 'greeting',
    templateText: 'Hello {{name}}, welcome to {{service}}!',
    variables: [
      { name: 'name', required: true },
      { name: 'service', required: true, defaultValue: 'our platform' }
    ]
  }
});

// Use template
const prompt = await client.instantiateTemplate({
  templateId: template.id,
  variableValues: {
    name: 'Alice',
    service: 'Support Chat'
  },
  sessionId: session.id
});

// Result: "Hello Alice, welcome to Support Chat!"
```

## CLI Operations

Use the CLI for database management:

```bash
# View stats
llm-memory-graph stats

# Query nodes
llm-memory-graph query -t prompt -l 10

# Get session details
llm-memory-graph session get <session-id>

# Export session
llm-memory-graph export session <session-id> -o backup.json

# Import data
llm-memory-graph import -i backup.json
```

## Common Patterns

### Chatbot Integration

```typescript
async function handleChatMessage(
  client: MemoryGraphClient,
  sessionId: string,
  userMessage: string
) {
  // Add user prompt
  const prompt = await client.addPrompt({
    sessionId,
    content: userMessage,
    metadata: { source: 'user' }
  });

  // Call your LLM
  const llmResponse = await callYourLLM(userMessage);

  // Add AI response
  const response = await client.addResponse({
    promptId: prompt.id,
    content: llmResponse.content,
    tokenUsage: llmResponse.usage,
    metadata: {
      model: llmResponse.model,
      finishReason: llmResponse.finishReason
    }
  });

  return response.content;
}
```

### Agent Workflow

```typescript
async function runAgentWorkflow(
  client: MemoryGraphClient,
  task: string
) {
  // Create session for task
  const session = await client.createSession({
    metadata: { task_type: 'agent_workflow' }
  });

  // Add initial prompt
  const prompt = await client.addPrompt({
    sessionId: session.id,
    content: task
  });

  // Agent processes task
  let currentPromptId = prompt.id;

  while (true) {
    // Get agent response
    const response = await getAgentResponse(task);

    await client.addResponse({
      promptId: currentPromptId,
      content: response.content,
      metadata: { agent: 'primary' }
    });

    // Check if tools needed
    if (response.toolCalls) {
      for (const toolCall of response.toolCalls) {
        const result = await executeTool(toolCall);

        await client.addToolInvocation({
          toolInvocation: {
            responseId: response.id,
            toolName: toolCall.name,
            parameters: toolCall.parameters,
            status: 'success',
            result,
            durationMs: result.duration
          }
        });
      }

      // Continue conversation
      const followUp = await client.addPrompt({
        sessionId: session.id,
        content: formatToolResults(response.toolCalls)
      });

      currentPromptId = followUp.id;
    } else {
      // Task complete
      break;
    }
  }

  return session.id;
}
```

### Multi-turn Conversation

```typescript
async function multiTurnConversation(
  client: MemoryGraphClient
) {
  // Create session
  const session = await client.createSession({
    metadata: { type: 'conversation' }
  });

  const turns = [
    { user: 'What is the capital of France?', assistant: 'Paris' },
    { user: 'What is its population?', assistant: 'About 2.2 million in the city' },
    { user: 'What are popular attractions?', assistant: 'Eiffel Tower, Louvre, Notre-Dame...' }
  ];

  for (const turn of turns) {
    const prompt = await client.addPrompt({
      sessionId: session.id,
      content: turn.user
    });

    await client.addResponse({
      promptId: prompt.id,
      content: turn.assistant
    });
  }

  // Get full conversation
  const nodes = await client.getSessionNodes(session.id);
  return nodes;
}
```

## Next Steps

### Learn More

- [API Documentation](../API.md) - Complete API reference
- [CLI Guide](../cli/README.md) - Command-line tool documentation
- [Examples](../EXAMPLES.md) - More code examples
- [Advanced Guide](advanced.md) - Advanced features and patterns

### Explore Features

- **Streaming** - Real-time event streaming
- **Templates** - Reusable prompt templates
- **Agents** - Multi-agent workflows
- **Metrics** - Performance monitoring
- **Export/Import** - Data portability

### Get Help

- [GitHub Issues](https://github.com/globalbusinessadvisors/llm-memory-graph/issues)
- [Documentation](https://github.com/globalbusinessadvisors/llm-memory-graph#readme)
- [Examples](https://github.com/globalbusinessadvisors/llm-memory-graph/tree/main/examples)

## Troubleshooting

### Connection Issues

```typescript
// Add retry policy
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  retryPolicy: {
    maxRetries: 3,
    initialBackoff: 100,
    maxBackoff: 5000
  }
});

// Check health
try {
  const health = await client.checkHealth();
  console.log('Connected:', health.status);
} catch (error) {
  console.error('Connection failed:', error.message);
}
```

### Server Not Running

```bash
# Check if server is running
llm-memory-graph stats

# If not, start it
llm-memory-graph server start

# Check logs
tail -f server.log
```

### Database Issues

```bash
# Verify database
llm-memory-graph verify

# Flush to disk
llm-memory-graph flush

# Check permissions
ls -la ./data
```

## Summary

You've learned how to:

1. Install and start the LLM Memory Graph server
2. Create a client and connect to the server
3. Create sessions and add prompts/responses
4. Query data and track tool invocations
5. Use templates for reusable prompts
6. Manage the database with CLI tools

Ready to build more advanced applications? Check out the [Advanced Guide](advanced.md) for more features and patterns.
