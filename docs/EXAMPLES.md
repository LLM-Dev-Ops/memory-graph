# Usage Examples

Comprehensive examples for common LLM Memory Graph use cases.

## Table of Contents

- [Basic Examples](#basic-examples)
- [Chatbot Integration](#chatbot-integration)
- [Agent Workflows](#agent-workflows)
- [Tool Invocation Tracking](#tool-invocation-tracking)
- [Template Usage](#template-usage)
- [Analytics and Reporting](#analytics-and-reporting)

## Basic Examples

### Simple Prompt-Response

```typescript
import { MemoryGraphClient } from '@llm-dev-ops/llm-memory-graph-client';

async function simpleExample() {
  const client = new MemoryGraphClient({
    address: 'localhost:50051'
  });

  try {
    const session = await client.createSession();

    const prompt = await client.addPrompt({
      sessionId: session.id,
      content: 'What is TypeScript?'
    });

    const response = await client.addResponse({
      promptId: prompt.id,
      content: 'TypeScript is a typed superset of JavaScript.'
    });

    console.log('Conversation saved:', response.id);
  } finally {
    await client.close();
  }
}
```

### Multi-turn Conversation

```typescript
async function multiTurn() {
  const client = new MemoryGraphClient({ address: 'localhost:50051' });
  const session = await client.createSession();

  const conversation = [
    { q: 'What is the capital of France?', a: 'Paris' },
    { q: 'What is its population?', a: 'About 2.2 million' },
    { q: 'What language do they speak?', a: 'French' }
  ];

  for (const turn of conversation) {
    const prompt = await client.addPrompt({
      sessionId: session.id,
      content: turn.q
    });

    await client.addResponse({
      promptId: prompt.id,
      content: turn.a
    });
  }

  const nodes = await client.getSessionNodes(session.id);
  console.log(`Saved ${nodes.length} nodes`);

  await client.close();
}
```

## Chatbot Integration

### Express.js Chatbot

```typescript
import express from 'express';
import { MemoryGraphClient } from '@llm-dev-ops/llm-memory-graph-client';

const app = express();
const client = new MemoryGraphClient({ address: 'localhost:50051' });

app.post('/chat', express.json(), async (req, res) => {
  const { sessionId, message } = req.body;

  try {
    // Add user prompt
    const prompt = await client.addPrompt({
      sessionId,
      content: message,
      metadata: { source: 'user' }
    });

    // Get AI response (from your LLM)
    const aiResponse = await getAIResponse(message);

    // Save response
    const response = await client.addResponse({
      promptId: prompt.id,
      content: aiResponse.content,
      tokenUsage: aiResponse.usage
    });

    res.json({ response: response.content });
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

app.listen(3000);
```

### Discord Bot

```typescript
import Discord from 'discord.js';
import { MemoryGraphClient } from '@llm-dev-ops/llm-memory-graph-client';

const discordClient = new Discord.Client();
const memoryClient = new MemoryGraphClient({ address: 'localhost:50051' });
const userSessions = new Map<string, string>();

discordClient.on('message', async (msg) => {
  if (msg.author.bot) return;

  // Get or create session for user
  let sessionId = userSessions.get(msg.author.id);
  if (!sessionId) {
    const session = await memoryClient.createSession({
      metadata: { userId: msg.author.id, platform: 'discord' }
    });
    sessionId = session.id;
    userSessions.set(msg.author.id, sessionId);
  }

  // Save user message
  const prompt = await memoryClient.addPrompt({
    sessionId,
    content: msg.content
  });

  // Generate and save bot response
  const botReply = await generateResponse(msg.content);
  await memoryClient.addResponse({
    promptId: prompt.id,
    content: botReply
  });

  msg.reply(botReply);
});

discordClient.login(process.env.DISCORD_TOKEN);
```

## Agent Workflows

### Research Agent

```typescript
async function researchAgent(client: MemoryGraphClient, topic: string) {
  const session = await client.createSession({
    metadata: { type: 'research', topic }
  });

  // Initial prompt
  const initialPrompt = await client.addPrompt({
    sessionId: session.id,
    content: `Research: ${topic}`
  });

  // Search tool invocation
  const searchResult = await performSearch(topic);
  await client.addToolInvocation({
    toolInvocation: {
      responseId: initialPrompt.id,
      toolName: 'web_search',
      parameters: { query: topic },
      status: 'success',
      result: searchResult,
      timestamp: new Date()
    }
  });

  // Analysis prompt
  const analysisPrompt = await client.addPrompt({
    sessionId: session.id,
    content: `Analyze results: ${JSON.stringify(searchResult)}`
  });

  // Final summary
  const summary = await generateSummary(searchResult);
  await client.addResponse({
    promptId: analysisPrompt.id,
    content: summary
  });

  return { sessionId: session.id, summary };
}
```

### Code Review Agent

```typescript
async function codeReviewAgent(
  client: MemoryGraphClient,
  code: string,
  language: string
) {
  const session = await client.createSession({
    metadata: { type: 'code_review', language }
  });

  // Use template
  const template = await client.getTemplate('code_review_template_id');
  const prompt = await client.instantiateTemplate({
    templateId: template.id,
    variableValues: { code, language },
    sessionId: session.id
  });

  // Lint tool
  const lintResult = await runLinter(code, language);
  await client.addToolInvocation({
    toolInvocation: {
      responseId: prompt.id,
      toolName: 'linter',
      parameters: { code, language },
      status: lintResult.success ? 'success' : 'failed',
      result: lintResult,
      timestamp: new Date()
    }
  });

  // AI review
  const review = await performAIReview(code, lintResult);
  await client.addResponse({
    promptId: prompt.id,
    content: review,
    metadata: { issues_found: lintResult.issues.length }
  });

  return review;
}
```

## Tool Invocation Tracking

### Database Query Tool

```typescript
async function trackDatabaseQuery(
  client: MemoryGraphClient,
  sessionId: string,
  responseId: string,
  query: string
) {
  const startTime = Date.now();

  try {
    const result = await database.query(query);
    const duration = Date.now() - startTime;

    await client.addToolInvocation({
      toolInvocation: {
        responseId,
        toolName: 'database_query',
        parameters: { query },
        status: 'success',
        result: { rows: result.rows.length, data: result.rows },
        durationMs: duration,
        timestamp: new Date()
      }
    });

    return result;
  } catch (error) {
    const duration = Date.now() - startTime;

    await client.addToolInvocation({
      toolInvocation: {
        responseId,
        toolName: 'database_query',
        parameters: { query },
        status: 'failed',
        error: error.message,
        durationMs: duration,
        timestamp: new Date()
      }
    });

    throw error;
  }
}
```

### API Call Tool

```typescript
async function trackAPICall(
  client: MemoryGraphClient,
  responseId: string,
  endpoint: string,
  params: any
) {
  const startTime = Date.now();

  try {
    const response = await fetch(endpoint, {
      method: 'POST',
      body: JSON.stringify(params)
    });
    const data = await response.json();
    const duration = Date.now() - startTime;

    await client.addToolInvocation({
      toolInvocation: {
        responseId,
        toolName: 'api_call',
        parameters: { endpoint, params },
        status: response.ok ? 'success' : 'failed',
        result: data,
        durationMs: duration,
        metadata: { statusCode: response.status }
      }
    });

    return data;
  } catch (error) {
    await client.addToolInvocation({
      toolInvocation: {
        responseId,
        toolName: 'api_call',
        parameters: { endpoint, params },
        status: 'failed',
        error: error.message,
        durationMs: Date.now() - startTime
      }
    });

    throw error;
  }
}
```

## Template Usage

### Email Templates

```typescript
async function createEmailTemplates(client: MemoryGraphClient) {
  // Welcome email
  const welcomeTemplate = await client.createTemplate({
    template: {
      name: 'welcome_email',
      templateText: `Hi {{name}},

Welcome to {{company}}! We're excited to have you.

{{custom_message}}

Best regards,
The {{company}} Team`,
      variables: [
        { name: 'name', required: true },
        { name: 'company', required: true },
        { name: 'custom_message', required: false, defaultValue: '' }
      ]
    }
  });

  // Use template
  const email = await client.instantiateTemplate({
    templateId: welcomeTemplate.id,
    variableValues: {
      name: 'Alice',
      company: 'Acme Corp',
      custom_message: 'Your account is ready!'
    }
  });

  return email.content;
}
```

### Prompt Templates

```typescript
async function createPromptTemplates(client: MemoryGraphClient) {
  // Code explanation template
  await client.createTemplate({
    template: {
      name: 'explain_code',
      templateText: `Explain this {{language}} code in simple terms:

\`\`\`{{language}}
{{code}}
\`\`\`

Focus on: {{focus}}`,
      variables: [
        { name: 'language', required: true },
        { name: 'code', required: true },
        { name: 'focus', required: false, defaultValue: 'purpose and key concepts' }
      ]
    }
  });

  // Translation template
  await client.createTemplate({
    template: {
      name: 'translate',
      templateText: 'Translate the following from {{from_lang}} to {{to_lang}}:\n\n{{text}}',
      variables: [
        { name: 'from_lang', required: true },
        { name: 'to_lang', required: true },
        { name: 'text', required: true }
      ]
    }
  });
}
```

## Analytics and Reporting

### Session Analytics

```typescript
async function analyzeSession(
  client: MemoryGraphClient,
  sessionId: string
) {
  const nodes = await client.getSessionNodes(sessionId);

  const prompts = nodes.filter(n => n.type === 'prompt');
  const responses = nodes.filter(n => n.type === 'response');
  const tools = nodes.filter(n => n.type === 'tool_invocation');

  const totalTokens = responses.reduce((sum, r) =>
    sum + (r.data.tokenUsage?.totalTokens || 0), 0
  );

  const avgLatency = responses.reduce((sum, r) =>
    sum + (r.data.metadata?.latencyMs || 0), 0
  ) / responses.length;

  return {
    promptCount: prompts.length,
    responseCount: responses.length,
    toolInvocations: tools.length,
    totalTokens,
    averageLatency: avgLatency,
    duration: new Date(nodes[nodes.length - 1].createdAt).getTime() -
              new Date(nodes[0].createdAt).getTime()
  };
}
```

### Usage Report

```typescript
async function generateUsageReport(
  client: MemoryGraphClient,
  startDate: Date,
  endDate: Date
) {
  const nodes = await client.queryNodes({
    after: startDate,
    before: endDate
  });

  const byType = nodes.nodes.reduce((acc, node) => {
    acc[node.type] = (acc[node.type] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  const byDay = nodes.nodes.reduce((acc, node) => {
    const day = node.createdAt.toISOString().split('T')[0];
    acc[day] = (acc[day] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  return {
    totalNodes: nodes.totalCount,
    byType,
    byDay,
    period: { start: startDate, end: endDate }
  };
}
```

### Token Usage Tracking

```typescript
async function trackTokenUsage(
  client: MemoryGraphClient,
  sessionId: string
) {
  const responses = await client.queryNodes({
    sessionId,
    nodeType: 'response'
  });

  const usage = responses.nodes.reduce((acc, node) => {
    const tokens = node.data.tokenUsage;
    if (tokens) {
      acc.prompt += tokens.promptTokens;
      acc.completion += tokens.completionTokens;
      acc.total += tokens.totalTokens;
    }
    return acc;
  }, { prompt: 0, completion: 0, total: 0 });

  const cost = calculateCost(usage);

  return { usage, cost };
}
```

## See Also

- [Quick Start Guide](guides/quickstart.md)
- [Advanced Guide](guides/advanced.md)
- [API Documentation](API.md)
- [CLI Examples](cli/examples.md)
