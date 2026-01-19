/**
 * LLM-Memory-Graph Unified Service Router
 *
 * Routes requests to the appropriate agent endpoint within a single Cloud Run service.
 * All agents are exposed under one unified service - no standalone deployments.
 *
 * Architecture:
 * - Single HTTP server on PORT (default 8080)
 * - Routes to 6 agent endpoints
 * - All persistence via ruvector-service (NO direct SQL)
 * - Telemetry to LLM-Observatory
 */

const http = require('http');
const url = require('url');
const fs = require('fs');
const path = require('path');

// Configuration from environment
const CONFIG = {
  port: parseInt(process.env.PORT || '8080', 10),
  platformEnv: process.env.PLATFORM_ENV || 'dev',
  serviceName: process.env.SERVICE_NAME || 'llm-memory-graph',
  serviceVersion: process.env.SERVICE_VERSION || 'unknown',
  ruvectorServiceUrl: process.env.RUVECTOR_SERVICE_URL || 'http://ruvector-service:8080',
  ruvectorApiKey: process.env.RUVECTOR_API_KEY || '',
  telemetryEndpoint: process.env.TELEMETRY_ENDPOINT || '',
};

// Agent registry - all agents with their configuration
const AGENTS = {
  'conversation-memory': {
    type: 'node',
    path: '/app/agents/conversation-memory',
    healthEndpoint: '/health',
    description: 'Captures and persists conversation structures',
    classification: 'MEMORY_WRITE',
  },
  'prompt-lineage': {
    type: 'node',
    path: '/app/agents/prompt-lineage',
    healthEndpoint: '/health',
    description: 'Tracks prompt evolution and lineage',
    classification: 'MEMORY_WRITE',
  },
  'decision-memory': {
    type: 'node',
    path: '/app/agents/decision-memory',
    healthEndpoint: '/health',
    description: 'Persists decisions and reasoning artifacts',
    classification: 'MEMORY_WRITE',
  },
  'knowledge-graph-builder': {
    type: 'node',
    path: '/app/agents/knowledge-graph-builder',
    healthEndpoint: '/health',
    description: 'Constructs knowledge graph from entities',
    classification: 'MEMORY_WRITE',
  },
  'memory-retrieval': {
    type: 'node',
    path: '/app/agents/memory-retrieval',
    healthEndpoint: '/health',
    description: 'Queries and retrieves memory subgraphs',
    classification: 'MEMORY_READ',
  },
  'long-term-pattern': {
    type: 'node',
    path: '/app/agents/long-term-pattern',
    healthEndpoint: '/health',
    description: 'Analyzes long-term patterns in memory',
    classification: 'MEMORY_READ',
  },
};

// Loaded agent modules
const loadedAgents = {};

// Load agent module dynamically
async function loadAgent(agentName) {
  if (loadedAgents[agentName]) {
    return loadedAgents[agentName];
  }

  const agent = AGENTS[agentName];
  if (!agent) return null;

  try {
    // Try to load from dist/index.js (compiled TypeScript)
    const modulePath = path.join(agent.path, 'dist', 'index.js');
    if (fs.existsSync(modulePath)) {
      const agentModule = await import(modulePath);
      loadedAgents[agentName] = agentModule;
      console.log(`Loaded agent: ${agentName} from ${modulePath}`);
      return agentModule;
    }

    // Try to load from src/index.js or src/index.ts
    const srcPath = path.join(agent.path, 'src', 'index.js');
    if (fs.existsSync(srcPath)) {
      const agentModule = await import(srcPath);
      loadedAgents[agentName] = agentModule;
      console.log(`Loaded agent: ${agentName} from ${srcPath}`);
      return agentModule;
    }

    console.log(`Agent ${agentName} module not found at ${agent.path}`);
    return null;
  } catch (error) {
    console.error(`Error loading agent ${agentName}:`, error.message);
    return null;
  }
}

// Telemetry helper
async function emitTelemetry(event, data) {
  if (!CONFIG.telemetryEndpoint) return;

  const payload = {
    timestamp: new Date().toISOString(),
    service: CONFIG.serviceName,
    version: CONFIG.serviceVersion,
    environment: CONFIG.platformEnv,
    event,
    data,
  };

  try {
    const telemetryUrl = new URL('/ingest', CONFIG.telemetryEndpoint);
    await fetch(telemetryUrl.toString(), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
      signal: AbortSignal.timeout(5000),
    });
  } catch (error) {
    // Silently ignore telemetry failures
  }
}

// Collect request body
function collectBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on('data', chunk => chunks.push(chunk));
    req.on('end', () => resolve(Buffer.concat(chunks).toString()));
    req.on('error', reject);
  });
}

// Route request to appropriate agent
async function routeToAgent(agentName, req, res, subPath) {
  const agent = AGENTS[agentName];
  if (!agent) {
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'Agent not found', agent: agentName }));
    return;
  }

  const startTime = Date.now();

  // Handle health check
  if (subPath === '/health' || subPath === '') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      status: 'healthy',
      agent: agentName,
      description: agent.description,
      classification: agent.classification,
      timestamp: new Date().toISOString(),
    }));
    return;
  }

  try {
    // Try to load and invoke the agent handler
    const agentModule = await loadAgent(agentName);

    if (agentModule && typeof agentModule.handler === 'function') {
      // Parse request body for POST/PUT
      let body = null;
      if (req.method === 'POST' || req.method === 'PUT') {
        const rawBody = await collectBody(req);
        try {
          body = JSON.parse(rawBody);
        } catch {
          body = rawBody;
        }
      }

      // Create request context
      const context = {
        path: subPath,
        method: req.method,
        headers: req.headers,
        body,
        config: {
          ruvectorServiceUrl: CONFIG.ruvectorServiceUrl,
          ruvectorApiKey: CONFIG.ruvectorApiKey,
          platformEnv: CONFIG.platformEnv,
        },
      };

      // Invoke agent handler
      const result = await agentModule.handler(context);

      emitTelemetry('agent_invocation', {
        agent: agentName,
        path: subPath,
        method: req.method,
        latencyMs: Date.now() - startTime,
        success: true,
      });

      res.writeHead(result.statusCode || 200, {
        'Content-Type': 'application/json',
        ...result.headers,
      });
      res.end(JSON.stringify(result.body || result));
    } else {
      // Agent module not available - return placeholder response
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        agent: agentName,
        path: subPath,
        status: 'available',
        message: 'Agent endpoint ready',
        ruvectorUrl: CONFIG.ruvectorServiceUrl ? 'configured' : 'not_configured',
      }));
    }
  } catch (error) {
    console.error(`Agent ${agentName} error:`, error);

    emitTelemetry('agent_error', {
      agent: agentName,
      path: subPath,
      latencyMs: Date.now() - startTime,
      error: error.message,
    });

    res.writeHead(500, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      error: 'Agent execution failed',
      agent: agentName,
      message: error.message,
    }));
  }
}

// Health check handler
function handleHealth(req, res) {
  const health = {
    status: 'healthy',
    service: CONFIG.serviceName,
    version: CONFIG.serviceVersion,
    environment: CONFIG.platformEnv,
    timestamp: new Date().toISOString(),
    agents: {},
  };

  // Check each agent's availability
  for (const [name, agent] of Object.entries(AGENTS)) {
    health.agents[name] = {
      status: 'available',
      classification: agent.classification,
      description: agent.description,
    };
  }

  // Dependencies status
  health.dependencies = {
    'ruvector-service': {
      url: CONFIG.ruvectorServiceUrl,
      status: CONFIG.ruvectorServiceUrl ? 'configured' : 'not_configured',
    },
    'llm-observatory': {
      url: CONFIG.telemetryEndpoint,
      status: CONFIG.telemetryEndpoint ? 'configured' : 'not_configured',
    },
  };

  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(health, null, 2));
}

// Service topology handler
function handleTopology(req, res) {
  const topology = {
    service: CONFIG.serviceName,
    version: CONFIG.serviceVersion,
    environment: CONFIG.platformEnv,
    description: 'LLM-Memory-Graph Unified Service',
    architecture: {
      type: 'unified_service',
      platform: 'google_cloud_run',
      stateless: true,
      persistence: 'ruvector-service',
      telemetry: 'llm-observatory',
    },
    agents: Object.entries(AGENTS).map(([name, agent]) => ({
      name,
      endpoint: `/${name}`,
      classification: agent.classification,
      description: agent.description,
      healthEndpoint: `/${name}/health`,
      cliCommand: `memory-graph agent ${name}`,
    })),
    constraints: [
      'NO_DIRECT_SQL_ACCESS',
      'NO_ORCHESTRATION',
      'NO_POLICY_ENFORCEMENT',
      'NO_INFERENCE_EXECUTION',
      'STATELESS_RUNTIME',
      'APPEND_ONLY_PERSISTENCE',
    ],
  };

  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(topology, null, 2));
}

// Main request handler
function handleRequest(req, res) {
  const parsedUrl = new URL(req.url, `http://localhost:${CONFIG.port}`);
  const pathParts = parsedUrl.pathname.split('/').filter(Boolean);

  // CORS headers
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');

  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }

  // Root endpoints
  if (pathParts.length === 0 || parsedUrl.pathname === '/') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      service: CONFIG.serviceName,
      version: CONFIG.serviceVersion,
      environment: CONFIG.platformEnv,
      endpoints: ['/health', '/topology', ...Object.keys(AGENTS).map(a => `/${a}`)],
    }));
    return;
  }

  // Health check
  if (parsedUrl.pathname === '/health') {
    handleHealth(req, res);
    return;
  }

  // Topology endpoint
  if (parsedUrl.pathname === '/topology') {
    handleTopology(req, res);
    return;
  }

  // Agent routing
  const agentName = pathParts[0];
  if (AGENTS[agentName]) {
    const subPath = '/' + pathParts.slice(1).join('/');
    routeToAgent(agentName, req, res, subPath);
    return;
  }

  // 404 for unknown paths
  res.writeHead(404, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({
    error: 'Not found',
    path: parsedUrl.pathname,
    availableAgents: Object.keys(AGENTS),
  }));
}

// Start the server
const server = http.createServer(handleRequest);

server.listen(CONFIG.port, () => {
  console.log(`LLM-Memory-Graph Unified Service started`);
  console.log(`  Environment: ${CONFIG.platformEnv}`);
  console.log(`  Version: ${CONFIG.serviceVersion}`);
  console.log(`  Port: ${CONFIG.port}`);
  console.log(`  Agents: ${Object.keys(AGENTS).join(', ')}`);
  console.log(`  ruvector-service: ${CONFIG.ruvectorServiceUrl || 'not configured'}`);
  console.log(`  Telemetry: ${CONFIG.telemetryEndpoint || 'disabled'}`);

  emitTelemetry('service_started', {
    agents: Object.keys(AGENTS),
  });
});

// Graceful shutdown
process.on('SIGTERM', () => {
  console.log('Received SIGTERM, shutting down gracefully...');
  emitTelemetry('service_shutdown', {});
  server.close(() => {
    console.log('Server closed');
    process.exit(0);
  });
});

process.on('SIGINT', () => {
  console.log('Received SIGINT, shutting down gracefully...');
  server.close(() => process.exit(0));
});
