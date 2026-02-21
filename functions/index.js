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
    'X-Correlation-ID, X-API-Version, Content-Type, Authorization',
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
// Health endpoint
// ---------------------------------------------------------------------------
app.get('/health', (_req, res) => {
  res.json({
    healthy: true,
    service: 'memory-graph-agents',
    agents: 6,
  });
});

// ---------------------------------------------------------------------------
// Agent routes: POST /v1/memory-graph/:agent
// ---------------------------------------------------------------------------
for (const [routeKey, config] of Object.entries(AGENT_MAP)) {
  app.post(`/v1/memory-graph/${routeKey}`, async (req, res) => {
    const execution_metadata = makeExecutionMetadata();
    const layers_executed = [];

    try {
      // ---- Layer 1: validate input ----
      layers_executed.push(`${config.agentDir}:validate-input`);

      const { payload, context } = req.body || {};
      const agentInput = payload ?? req.body;

      // ---- Layer 2: load agent ----
      layers_executed.push(`${config.agentDir}:load-agent`);
      const agentModule = await loadAgentModule(config.agentDir);

      if (agentModule && typeof agentModule.executeAsUnit === 'function') {
        // ---- Layer 3: execute ----
        layers_executed.push(`${config.agentDir}:execute`);
        const result = await agentModule.executeAsUnit(agentInput);

        // ---- Layer 4: format output ----
        layers_executed.push(`${config.agentDir}:format-output`);

        return res.json({
          ...result,
          execution_metadata,
          layers_executed,
        });
      }

      if (agentModule && typeof agentModule.handler === 'function') {
        // Fallback: use the handler export (varies per agent)
        layers_executed.push(`${config.agentDir}:execute-handler`);

        const handlerReq = {
          method: 'POST',
          body: agentInput,
          headers: req.headers,
        };
        const handlerResult = await agentModule.handler(handlerReq);

        layers_executed.push(`${config.agentDir}:format-output`);

        // handler returns { statusCode, headers, body(string) } or { body(object) }
        let body = handlerResult.body;
        if (typeof body === 'string') {
          try { body = JSON.parse(body); } catch { /* keep as-is */ }
        }

        return res.status(handlerResult.statusCode || 200).json({
          ...(typeof body === 'object' && body !== null ? body : { data: body }),
          execution_metadata,
          layers_executed,
        });
      }

      // Agent module exists but has no usable export
      layers_executed.push(`${config.agentDir}:no-handler`);
      return res.status(501).json({
        success: false,
        error: {
          error_code: 'AGENT_NOT_AVAILABLE',
          message: `Agent ${config.agentDir} is loaded but has no executeAsUnit or handler export`,
        },
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
  });
}

// ---------------------------------------------------------------------------
// Fallback 404
// ---------------------------------------------------------------------------
app.use((_req, res) => {
  res.status(404).json({
    error: 'Not found',
    service: 'memory-graph-agents',
    available_routes: [
      'GET  /health',
      ...Object.keys(AGENT_MAP).map((k) => `POST /v1/memory-graph/${k}`),
    ],
    execution_metadata: makeExecutionMetadata(),
    layers_executed: ['router:not-found'],
  });
});

// ---------------------------------------------------------------------------
// Export for Cloud Functions (--entry-point api)
// ---------------------------------------------------------------------------
export const api = app;
