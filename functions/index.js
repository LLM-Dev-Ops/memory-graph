/**
 * Cloud Function entry point for memory-graph-agents
 *
 * Deployable via:
 *   gcloud functions deploy memory-graph-agents \
 *     --runtime nodejs20 --trigger-http --allow-unauthenticated \
 *     --region us-central1 --project agentics-dev \
 *     --entry-point api --memory 512MB --timeout 60s
 *
 * Exposes 6 agent routes under /v1/memory-graph/ plus a /health endpoint.
 * Every response includes execution_metadata and layers_executed per the
 * upstream CLI contract validator requirements.
 */

import express from 'express';
import { randomUUID } from 'node:crypto';

const app = express();

// ---------------------------------------------------------------------------
// CORS - allow required headers
// ---------------------------------------------------------------------------
app.use((req, res, next) => {
  res.set('Access-Control-Allow-Origin', '*');
  res.set('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.set(
    'Access-Control-Allow-Headers',
    'X-Correlation-ID, X-API-Version, X-Agent-ID, X-Agent-Version, Content-Type, Authorization',
  );
  if (req.method === 'OPTIONS') {
    return res.status(204).send('');
  }
  next();
});

app.use(express.json());

// ---------------------------------------------------------------------------
// execution_metadata helper
// ---------------------------------------------------------------------------
function makeExecutionMetadata() {
  return {
    trace_id: randomUUID(),
    timestamp: new Date().toISOString(),
    service: 'memory-graph-agents',
  };
}

// ---------------------------------------------------------------------------
// Agent module cache (lazy-loaded)
// ---------------------------------------------------------------------------
const agentModules = {};

/**
 * Route name  -> agent directory name mapping.
 * Route:  POST /v1/memory-graph/<routeKey>
 * Agent dir:  agents/<agentDir>/dist/index.js
 */
const AGENT_MAP = {
  conversation:      { agentDir: 'conversation-memory',      label: 'Conversation Memory' },
  lineage:           { agentDir: 'prompt-lineage',           label: 'Prompt Lineage' },
  decisions:         { agentDir: 'decision-memory',          label: 'Decision Memory' },
  'knowledge-graph': { agentDir: 'knowledge-graph-builder',  label: 'Knowledge Graph Builder' },
  retrieval:         { agentDir: 'memory-retrieval',         label: 'Memory Retrieval' },
  patterns:          { agentDir: 'long-term-pattern',        label: 'Long-Term Pattern' },
};

/**
 * Try to load a TS agent module from agents/<dir>/dist/index.js.
 * Returns the module or null if not found.
 */
async function loadAgentModule(agentDir) {
  if (agentModules[agentDir]) return agentModules[agentDir];

  const paths = [
    `../agents/${agentDir}/dist/index.js`,
    `../agents/${agentDir}/src/index.js`,
  ];

  for (const p of paths) {
    try {
      const mod = await import(p);
      agentModules[agentDir] = mod;
      return mod;
    } catch {
      // try next path
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Health handler (shared by /health and /api/v1/health)
// ---------------------------------------------------------------------------
function healthHandler(_req, res) {
  res.json({
    healthy: true,
    service: 'memory-graph-agents',
    agents: 6,
  });
}

app.get('/health', healthHandler);
app.get('/api/v1/health', healthHandler);

// ---------------------------------------------------------------------------
// Invoke an agent module with the given input. Returns a normalized
// { status, body, subLayers } tuple so request handlers can compose.
// ---------------------------------------------------------------------------
async function invokeAgent(agentDir, agentInput) {
  const subLayers = [`${agentDir}:load-agent`];
  const agentModule = await loadAgentModule(agentDir);

  if (agentModule && typeof agentModule.executeAsUnit === 'function') {
    subLayers.push(`${agentDir}:execute`);
    const result = await agentModule.executeAsUnit(agentInput);
    subLayers.push(`${agentDir}:format-output`);
    return { status: 200, body: result, subLayers };
  }

  if (agentModule && typeof agentModule.handler === 'function') {
    subLayers.push(`${agentDir}:execute-handler`);
    const handlerResult = await agentModule.handler({
      method: 'POST',
      body: agentInput,
      headers: {},
    });
    subLayers.push(`${agentDir}:format-output`);

    let body = handlerResult.body;
    if (typeof body === 'string') {
      try { body = JSON.parse(body); } catch { /* keep as-is */ }
    }
    return {
      status: handlerResult.statusCode || 200,
      body: (typeof body === 'object' && body !== null) ? body : { data: body },
      subLayers,
    };
  }

  subLayers.push(`${agentDir}:no-handler`);
  return {
    status: 501,
    body: {
      success: false,
      error: {
        error_code: 'AGENT_NOT_AVAILABLE',
        message: `Agent ${agentDir} is loaded but has no executeAsUnit or handler export`,
      },
    },
    subLayers,
  };
}

// ---------------------------------------------------------------------------
// Agent route handler factory (POST /v1/memory-graph/:agent)
// ---------------------------------------------------------------------------
function makeAgentHandler(config) {
  return async (req, res) => {
    const execution_metadata = makeExecutionMetadata();
    const layers_executed = [`${config.agentDir}:validate-input`];

    try {
      const { payload } = req.body || {};
      const agentInput = payload ?? req.body;

      const { status, body, subLayers } = await invokeAgent(config.agentDir, agentInput);
      layers_executed.push(...subLayers);

      return res.status(status).json({
        ...body,
        execution_metadata,
        layers_executed,
      });
    } catch (err) {
      layers_executed.push(`${config.agentDir}:error`);
      return res.status(500).json({
        success: false,
        error: {
          error_code: 'INTERNAL_ERROR',
          message: err.message || 'Unknown error',
        },
        execution_metadata,
        layers_executed,
      });
    }
  };
}

// ---------------------------------------------------------------------------
// Decision store (ephemeral, per-instance, bounded FIFO).
//
// The canonical Rust decision-memory crate has no JS build output, so the
// router cannot invoke it in-process. Until a Firestore-backed durable store
// lands, this Map keeps decisions reachable from the instance that wrote
// them — which is enough for the upstream POST callers that don't read back.
// Cross-instance GET is not guaranteed.
// ---------------------------------------------------------------------------
const DECISION_STORE_CAP = 10000;
const decisionStore = new Map();

function persistDecision(payload) {
  const id = randomUUID();
  const created_at = new Date().toISOString();
  const record = { id, created_at, ...payload };
  decisionStore.set(id, record);
  if (decisionStore.size > DECISION_STORE_CAP) {
    const oldest = decisionStore.keys().next().value;
    decisionStore.delete(oldest);
  }
  return record;
}

function queryDecisions(query = {}) {
  const { agent_id, decision_type, execution_ref, start_time, end_time } = query;
  let items = Array.from(decisionStore.values());

  if (agent_id)      items = items.filter((r) => r.agent_id === agent_id);
  if (decision_type) items = items.filter((r) => r.decision_type === decision_type);
  if (execution_ref) items = items.filter((r) => r.execution_ref === execution_ref);
  if (start_time) {
    const ts = Date.parse(start_time);
    if (!Number.isNaN(ts)) items = items.filter((r) => Date.parse(r.created_at) >= ts);
  }
  if (end_time) {
    const ts = Date.parse(end_time);
    if (!Number.isNaN(ts)) items = items.filter((r) => Date.parse(r.created_at) <= ts);
  }

  const total = items.length;
  const offset = Math.max(0, parseInt(query.offset, 10) || 0);
  const limit = Math.max(1, Math.min(1000, parseInt(query.limit, 10) || 100));
  items = items.slice(offset, offset + limit);
  return { items, total };
}

// ---------------------------------------------------------------------------
// Decision handlers — shared by /v1/memory-graph/decisions and the
// /api/v1/decisions alias family. Pure in-memory, no agent module invoked.
// ---------------------------------------------------------------------------
function decisionCaptureHandler(req, res) {
  const execution_metadata = makeExecutionMetadata();
  const layers_executed = ['decision-memory:validate-input', 'decision-memory:persist'];

  const body = req.body;
  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    return res.status(400).json({
      success: false,
      error: {
        error_code: 'INVALID_INPUT',
        message: 'Body must be a DecisionEvent object',
      },
      execution_metadata,
      layers_executed,
    });
  }

  const record = persistDecision(body);
  return res.status(200).json({
    ...record,
    execution_metadata,
    layers_executed,
  });
}

function decisionBatchHandler(req, res) {
  const execution_metadata = makeExecutionMetadata();
  const layers_executed = ['decision-memory:validate-input-batch', 'decision-memory:persist-batch'];

  const body = req.body || {};
  const items = Array.isArray(body) ? body : (body.decisions ?? body.items ?? []);

  if (!Array.isArray(items) || items.length === 0) {
    return res.status(400).json({
      success: false,
      error: {
        error_code: 'INVALID_INPUT',
        message: 'Batch requires an array body, or { decisions: [...] } / { items: [...] } with ≥1 entry',
      },
      execution_metadata,
      layers_executed,
    });
  }

  const results = items.map((item) => {
    if (!item || typeof item !== 'object' || Array.isArray(item)) {
      return {
        success: false,
        error: { error_code: 'INVALID_INPUT', message: 'Each batch item must be a DecisionEvent object' },
      };
    }
    const record = persistDecision(item);
    return { success: true, ...record };
  });

  return res.status(200).json({
    count: results.length,
    items: results,
    execution_metadata,
    layers_executed,
  });
}

function decisionQueryHandler(req, res) {
  const execution_metadata = makeExecutionMetadata();
  const layers_executed = ['decision-memory:validate-input-query', 'decision-memory:query'];

  const { items, total } = queryDecisions(req.query || {});
  return res.status(200).json({
    items,
    total,
    execution_metadata,
    layers_executed,
  });
}

// ---------------------------------------------------------------------------
// Agent routes: POST /v1/memory-graph/:agent
// decisions is intercepted so the in-memory handler runs in place of the
// (non-existent) JS agent module.
// ---------------------------------------------------------------------------
for (const [routeKey, config] of Object.entries(AGENT_MAP)) {
  if (routeKey === 'decisions') {
    app.post(`/v1/memory-graph/${routeKey}`, decisionCaptureHandler);
    continue;
  }
  app.post(`/v1/memory-graph/${routeKey}`, makeAgentHandler(config));
}

// ---------------------------------------------------------------------------
// /api/v1 alias surface — unblocks upstream callers (auto-optimizer-agents,
// costops-agents) hitting RUVECTOR_SERVICE_URL with the /api/v1 shape.
// ---------------------------------------------------------------------------
app.post('/api/v1/decisions', decisionCaptureHandler);
app.post('/api/v1/decisions/batch', decisionBatchHandler);
app.get('/api/v1/decisions', decisionQueryHandler);

// Telemetry sink used by costops-agents. Fire-and-forget: accept, ack, return.
app.post('/events', (req, res) => {
  const execution_metadata = makeExecutionMetadata();
  const body = req.body || {};
  const payloadBytes = (() => {
    try { return JSON.stringify(body).length; } catch { return -1; }
  })();

  return res.status(202).json({
    success: true,
    event_id: randomUUID(),
    received: {
      type: body.type || body.event_type || null,
      payload_size_bytes: payloadBytes,
    },
    execution_metadata,
    layers_executed: ['events:accept'],
  });
});

// ---------------------------------------------------------------------------
// Fallback 404
// ---------------------------------------------------------------------------
app.use((_req, res) => {
  res.status(404).json({
    error: 'Not found',
    service: 'memory-graph-agents',
    available_routes: [
      'GET  /health',
      'GET  /api/v1/health',
      ...Object.keys(AGENT_MAP).map((k) => `POST /v1/memory-graph/${k}`),
      'POST /api/v1/decisions',
      'POST /api/v1/decisions/batch',
      'GET  /api/v1/decisions',
      'POST /events',
    ],
    execution_metadata: makeExecutionMetadata(),
    layers_executed: ['router:not-found'],
  });
});

// ---------------------------------------------------------------------------
// Export for Cloud Functions (--entry-point api)
// ---------------------------------------------------------------------------
export const api = app;
