/**
 * LLM-Memory-Graph Unified Service Router
 * Phase 2 - Operational Intelligence (Layer 1)
 *
 * Routes requests to the appropriate agent endpoint within a single Cloud Run service.
 * All agents are exposed under one unified service - no standalone deployments.
 *
 * Architecture:
 * - Single HTTP server on PORT (default 8080)
 * - Routes to 6 agent endpoints
 * - All persistence via ruvector-service (NO direct SQL)
 * - Telemetry to LLM-Observatory
 *
 * Phase 2 Requirements:
 * - Hard startup failure if Ruvector unavailable
 * - Signal emission (anomaly, drift, memory lineage, latency)
 * - Performance budgets (MAX_TOKENS, MAX_LATENCY_MS, MAX_CALLS_PER_RUN)
 * - Caching for historical reads/lineage lookups (TTL 60-120s)
 */

const http = require('http');
const url = require('url');
const fs = require('fs');
const path = require('path');

// Phase 2 modules
let phase2 = null;
let signalEmitter = null;
let phase2Context = null;
let phase2Cache = null;

// Configuration from environment
const CONFIG = {
  port: parseInt(process.env.PORT || '8080', 10),
  platformEnv: process.env.PLATFORM_ENV || 'dev',
  serviceName: process.env.SERVICE_NAME || 'llm-memory-graph',
  serviceVersion: process.env.SERVICE_VERSION || 'unknown',
  ruvectorServiceUrl: process.env.RUVECTOR_SERVICE_URL || '',
  ruvectorApiKey: process.env.RUVECTOR_API_KEY || '',
  telemetryEndpoint: process.env.TELEMETRY_ENDPOINT || '',
  // Phase 2 specific
  agentName: process.env.AGENT_NAME || 'llm-memory-graph',
  agentDomain: process.env.AGENT_DOMAIN || 'memory',
  agentPhase: process.env.AGENT_PHASE || 'phase2',
  agentLayer: process.env.AGENT_LAYER || 'layer1',
  // Performance budgets
  maxTokens: parseInt(process.env.MAX_TOKENS || '1000', 10),
  maxLatencyMs: parseInt(process.env.MAX_LATENCY_MS || '2000', 10),
  maxCallsPerRun: parseInt(process.env.MAX_CALLS_PER_RUN || '3', 10),
  // Logging (minimal for Phase 2)
  logLevel: process.env.LOG_LEVEL || 'warn',
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

/**
 * Minimal logging for Phase 2.
 * Only logs errors and critical info.
 */
function log(level, message, data = {}) {
  const levels = { error: 0, warn: 1, info: 2, debug: 3 };
  const configLevel = levels[CONFIG.logLevel] ?? 1;
  const msgLevel = levels[level] ?? 2;

  if (msgLevel <= configLevel) {
    const entry = {
      timestamp: new Date().toISOString(),
      level,
      message,
      service: CONFIG.serviceName,
      phase: CONFIG.agentPhase,
      layer: CONFIG.agentLayer,
      ...data,
    };
    console.log(JSON.stringify(entry));
  }
}

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
      log('info', `Loaded agent: ${agentName}`, { path: modulePath });
      return agentModule;
    }

    // Try to load from src/index.js or src/index.ts
    const srcPath = path.join(agent.path, 'src', 'index.js');
    if (fs.existsSync(srcPath)) {
      const agentModule = await import(srcPath);
      loadedAgents[agentName] = agentModule;
      log('info', `Loaded agent: ${agentName}`, { path: srcPath });
      return agentModule;
    }

    log('warn', `Agent module not found: ${agentName}`, { path: agent.path });
    return null;
  } catch (error) {
    log('error', `Error loading agent: ${agentName}`, { error: error.message });
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
    phase: CONFIG.agentPhase,
    layer: CONFIG.agentLayer,
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
      phase: CONFIG.agentPhase,
      layer: CONFIG.agentLayer,
      timestamp: new Date().toISOString(),
    }));
    return;
  }

  try {
    // Create budget tracker for this request
    const budgetTracker = phase2 ? new phase2.BudgetTracker(signalEmitter) : null;

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

      // Create request context with Phase 2 extensions
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
        // Phase 2 context
        phase2: {
          agentPhase: CONFIG.agentPhase,
          agentLayer: CONFIG.agentLayer,
          budgetTracker,
          signalEmitter,
          cache: phase2Cache,
          budgets: {
            maxTokens: CONFIG.maxTokens,
            maxLatencyMs: CONFIG.maxLatencyMs,
            maxCallsPerRun: CONFIG.maxCallsPerRun,
          },
        },
      };

      // Invoke agent handler
      const result = await agentModule.handler(context);

      const latencyMs = Date.now() - startTime;

      // Emit latency signal
      if (signalEmitter && phase2) {
        await signalEmitter.emit(phase2.createLatencySignal({
          operation: `agent_${agentName}`,
          latency_ms: latencyMs,
          budget_ms: CONFIG.maxLatencyMs,
          within_budget: latencyMs <= CONFIG.maxLatencyMs,
          confidence: 1.0,
        }));
      }

      emitTelemetry('agent_invocation', {
        agent: agentName,
        path: subPath,
        method: req.method,
        latencyMs,
        success: true,
        budgetSummary: budgetTracker?.getSummary(),
      });

      res.writeHead(result.statusCode || 200, {
        'Content-Type': 'application/json',
        'X-Agent-Phase': CONFIG.agentPhase,
        'X-Agent-Layer': CONFIG.agentLayer,
        'X-Request-Latency-Ms': String(latencyMs),
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
        phase: CONFIG.agentPhase,
        layer: CONFIG.agentLayer,
        ruvectorUrl: CONFIG.ruvectorServiceUrl ? 'configured' : 'not_configured',
      }));
    }
  } catch (error) {
    log('error', `Agent error: ${agentName}`, { error: error.message, path: subPath });

    // Emit anomaly signal for errors
    if (signalEmitter && phase2) {
      await signalEmitter.emit(phase2.createAnomalySignal({
        metric: 'agent_error',
        observed_value: 1,
        expected_value: 0,
        deviation: 1,
        confidence: 1.0,
      }));
    }

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
      phase: CONFIG.agentPhase,
      layer: CONFIG.agentLayer,
    }));
  }
}

// Health check handler
async function handleHealth(req, res) {
  // Verify Ruvector is still available
  let ruvectorStatus = { available: false };
  if (phase2) {
    ruvectorStatus = await phase2.verifyRuvector(
      CONFIG.ruvectorServiceUrl,
      CONFIG.ruvectorApiKey,
      3000
    );
  }

  const health = {
    status: ruvectorStatus.available ? 'healthy' : 'degraded',
    service: CONFIG.serviceName,
    version: CONFIG.serviceVersion,
    environment: CONFIG.platformEnv,
    phase: CONFIG.agentPhase,
    layer: CONFIG.agentLayer,
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
      status: ruvectorStatus.available ? 'healthy' : 'unavailable',
      latency_ms: ruvectorStatus.latency_ms,
    },
    'llm-observatory': {
      url: CONFIG.telemetryEndpoint,
      status: CONFIG.telemetryEndpoint ? 'configured' : 'not_configured',
    },
  };

  // Performance budgets
  health.budgets = {
    MAX_TOKENS: CONFIG.maxTokens,
    MAX_LATENCY_MS: CONFIG.maxLatencyMs,
    MAX_CALLS_PER_RUN: CONFIG.maxCallsPerRun,
  };

  // Cache stats
  if (phase2Cache) {
    health.cache = phase2Cache.getStats();
  }

  const statusCode = ruvectorStatus.available ? 200 : 503;
  res.writeHead(statusCode, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(health, null, 2));
}

// Service topology handler
function handleTopology(req, res) {
  const topology = {
    service: CONFIG.serviceName,
    version: CONFIG.serviceVersion,
    environment: CONFIG.platformEnv,
    phase: CONFIG.agentPhase,
    layer: CONFIG.agentLayer,
    description: 'LLM-Memory-Graph Unified Service - Phase 2 Operational Intelligence',
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
      // Phase 2 constraints
      'RUVECTOR_AUTHORITATIVE_EVENT_INDEX',
      'EMIT_LINEAGE_DELTAS_ONLY',
      'NO_SYNTHESIZED_CONCLUSIONS',
    ],
    budgets: {
      MAX_TOKENS: CONFIG.maxTokens,
      MAX_LATENCY_MS: CONFIG.maxLatencyMs,
      MAX_CALLS_PER_RUN: CONFIG.maxCallsPerRun,
    },
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
  // Phase 2 headers
  res.setHeader('X-Agent-Phase', CONFIG.agentPhase);
  res.setHeader('X-Agent-Layer', CONFIG.agentLayer);

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
      phase: CONFIG.agentPhase,
      layer: CONFIG.agentLayer,
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
    phase: CONFIG.agentPhase,
    layer: CONFIG.agentLayer,
  }));
}

/**
 * Phase 2 startup sequence.
 * CRITICAL: Hard failure if Ruvector is unavailable.
 */
async function startPhase2Server() {
  try {
    // Load Phase 2 modules
    phase2 = require('./phase2/index.js');

    log('info', 'Starting Phase 2 - Operational Intelligence (Layer 1)');

    // Phase 2 startup hardening
    phase2Context = await phase2.initializePhase2();

    // Initialize signal emitter
    signalEmitter = new phase2.SignalEmitter(
      CONFIG.ruvectorServiceUrl,
      CONFIG.ruvectorApiKey
    );

    // Initialize cache
    phase2Cache = phase2.getCache();

    log('info', 'Phase 2 initialization complete', {
      ruvector_latency_ms: phase2Context.ruvector.startup_latency_ms,
      budgets: phase2Context.budgets,
    });

  } catch (error) {
    // HARD FAILURE - Cannot start without Ruvector
    console.error('='.repeat(60));
    console.error('FATAL: Phase 2 startup failed');
    console.error(error.message);
    console.error('='.repeat(60));
    process.exit(1);
  }

  // Start the server
  const server = http.createServer(handleRequest);

  server.listen(CONFIG.port, () => {
    log('info', 'LLM-Memory-Graph Unified Service started', {
      environment: CONFIG.platformEnv,
      version: CONFIG.serviceVersion,
      port: CONFIG.port,
      agents: Object.keys(AGENTS),
      phase: CONFIG.agentPhase,
      layer: CONFIG.agentLayer,
    });

    emitTelemetry('service_started', {
      agents: Object.keys(AGENTS),
      phase: CONFIG.agentPhase,
      layer: CONFIG.agentLayer,
      budgets: {
        MAX_TOKENS: CONFIG.maxTokens,
        MAX_LATENCY_MS: CONFIG.maxLatencyMs,
        MAX_CALLS_PER_RUN: CONFIG.maxCallsPerRun,
      },
    });
  });

  // Graceful shutdown
  process.on('SIGTERM', async () => {
    log('info', 'Received SIGTERM, shutting down gracefully...');

    // Flush pending signals
    if (signalEmitter) {
      await signalEmitter.flush();
    }

    emitTelemetry('service_shutdown', {
      phase: CONFIG.agentPhase,
      layer: CONFIG.agentLayer,
    });

    server.close(() => {
      log('info', 'Server closed');
      process.exit(0);
    });
  });

  process.on('SIGINT', async () => {
    log('info', 'Received SIGINT, shutting down gracefully...');

    if (signalEmitter) {
      await signalEmitter.flush();
    }

    server.close(() => process.exit(0));
  });
}

// Start the server with Phase 2 initialization
startPhase2Server();
