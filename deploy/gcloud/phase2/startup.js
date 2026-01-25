/**
 * Phase 2 - Operational Intelligence (Layer 1)
 * Startup Hardening Module
 *
 * CRITICAL: Enforces hard startup failure if Ruvector is unavailable.
 * All required environment variables must be present.
 */

const REQUIRED_ENV_VARS = [
  'RUVECTOR_SERVICE_URL',
  'RUVECTOR_API_KEY',
  'AGENT_NAME',
  'AGENT_DOMAIN',
  'AGENT_PHASE',
  'AGENT_LAYER',
];

const PHASE2_DEFAULTS = {
  AGENT_PHASE: 'phase2',
  AGENT_LAYER: 'layer1',
};

/**
 * Validates all required environment variables are present.
 * Throws on missing required vars.
 *
 * @returns {Object} Validated configuration
 */
function validateEnvironment() {
  const errors = [];
  const config = {};

  for (const varName of REQUIRED_ENV_VARS) {
    const value = process.env[varName] || PHASE2_DEFAULTS[varName];
    if (!value) {
      errors.push(`Missing required environment variable: ${varName}`);
    } else {
      config[varName] = value;
    }
  }

  // Validate AGENT_PHASE
  if (config.AGENT_PHASE && config.AGENT_PHASE !== 'phase2') {
    errors.push(`AGENT_PHASE must be 'phase2', got: ${config.AGENT_PHASE}`);
  }

  // Validate AGENT_LAYER
  if (config.AGENT_LAYER && config.AGENT_LAYER !== 'layer1') {
    errors.push(`AGENT_LAYER must be 'layer1', got: ${config.AGENT_LAYER}`);
  }

  if (errors.length > 0) {
    const errorMessage = [
      '='.repeat(60),
      'PHASE 2 STARTUP FAILURE - Missing Required Configuration',
      '='.repeat(60),
      ...errors,
      '',
      'Required environment variables:',
      ...REQUIRED_ENV_VARS.map(v => `  - ${v}`),
      '='.repeat(60),
    ].join('\n');

    console.error(errorMessage);
    throw new Error(`Startup validation failed: ${errors.join('; ')}`);
  }

  return config;
}

/**
 * Verifies Ruvector service is available.
 * HARD FAILURE if Ruvector is unreachable.
 *
 * @param {string} ruvectorUrl - Ruvector service URL
 * @param {string} apiKey - Ruvector API key
 * @param {number} timeoutMs - Timeout in milliseconds
 * @returns {Promise<{available: boolean, latency_ms: number}>}
 */
async function verifyRuvector(ruvectorUrl, apiKey, timeoutMs = 5000) {
  const startTime = Date.now();

  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

    const response = await fetch(`${ruvectorUrl}/health`, {
      method: 'GET',
      headers: {
        'Authorization': `Bearer ${apiKey}`,
        'Content-Type': 'application/json',
        'X-Agent-Phase': 'phase2',
        'X-Agent-Layer': 'layer1',
      },
      signal: controller.signal,
    });

    clearTimeout(timeoutId);

    const latency_ms = Date.now() - startTime;

    if (!response.ok) {
      throw new Error(`Ruvector health check failed: HTTP ${response.status}`);
    }

    return { available: true, latency_ms };
  } catch (error) {
    const latency_ms = Date.now() - startTime;

    console.error([
      '='.repeat(60),
      'PHASE 2 STARTUP FAILURE - Ruvector Unavailable',
      '='.repeat(60),
      `URL: ${ruvectorUrl}`,
      `Error: ${error.message}`,
      `Latency: ${latency_ms}ms`,
      '',
      'Ruvector is REQUIRED for Phase 2 operation.',
      'Service cannot start without Ruvector connectivity.',
      '='.repeat(60),
    ].join('\n'));

    return { available: false, latency_ms, error: error.message };
  }
}

/**
 * Phase 2 startup initialization.
 * Validates environment and verifies Ruvector connectivity.
 *
 * @returns {Promise<Object>} Startup context
 */
async function initializePhase2() {
  console.log('Phase 2 - Operational Intelligence (Layer 1) Startup');
  console.log('='.repeat(50));

  // Step 1: Validate environment
  console.log('[1/3] Validating environment variables...');
  const config = validateEnvironment();
  console.log(`      AGENT_NAME: ${config.AGENT_NAME}`);
  console.log(`      AGENT_DOMAIN: ${config.AGENT_DOMAIN}`);
  console.log(`      AGENT_PHASE: ${config.AGENT_PHASE}`);
  console.log(`      AGENT_LAYER: ${config.AGENT_LAYER}`);

  // Step 2: Verify Ruvector connectivity
  console.log('[2/3] Verifying Ruvector connectivity...');
  const ruvectorStatus = await verifyRuvector(
    config.RUVECTOR_SERVICE_URL,
    config.RUVECTOR_API_KEY
  );

  if (!ruvectorStatus.available) {
    throw new Error(`Ruvector unavailable: ${ruvectorStatus.error}`);
  }
  console.log(`      Ruvector available (latency: ${ruvectorStatus.latency_ms}ms)`);

  // Step 3: Initialize Phase 2 context
  console.log('[3/3] Initializing Phase 2 context...');
  const phase2Context = {
    ...config,
    ruvector: {
      url: config.RUVECTOR_SERVICE_URL,
      available: true,
      startup_latency_ms: ruvectorStatus.latency_ms,
    },
    startup_timestamp: new Date().toISOString(),
    budgets: {
      MAX_TOKENS: parseInt(process.env.MAX_TOKENS || '1000', 10),
      MAX_LATENCY_MS: parseInt(process.env.MAX_LATENCY_MS || '2000', 10),
      MAX_CALLS_PER_RUN: parseInt(process.env.MAX_CALLS_PER_RUN || '3', 10),
    },
    cache: {
      TTL_MIN_S: 60,
      TTL_MAX_S: 120,
    },
  };

  console.log('='.repeat(50));
  console.log('Phase 2 startup complete');

  return phase2Context;
}

module.exports = {
  validateEnvironment,
  verifyRuvector,
  initializePhase2,
  REQUIRED_ENV_VARS,
  PHASE2_DEFAULTS,
};
