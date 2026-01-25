/**
 * Phase 2 - Operational Intelligence (Layer 1)
 * Signal Emission Module
 *
 * Agents MUST emit:
 * - anomaly signals
 * - drift signals
 * - memory lineage signals
 * - latency signals
 *
 * Signals MUST:
 * - Be atomic
 * - Include confidence
 * - Avoid conclusions (raw observations only)
 */

/**
 * Signal types for Phase 2 Layer 1
 */
const SignalType = {
  ANOMALY: 'anomaly',
  DRIFT: 'drift',
  MEMORY_LINEAGE: 'memory_lineage',
  LATENCY: 'latency',
};

/**
 * Creates an atomic signal with confidence.
 * Signals are raw observations - NO synthesized conclusions.
 *
 * @param {string} type - Signal type (anomaly|drift|memory_lineage|latency)
 * @param {Object} observation - Raw observation data
 * @param {number} confidence - Confidence score (0.0 - 1.0)
 * @param {Object} metadata - Additional metadata
 * @returns {Object} Atomic signal
 */
function createSignal(type, observation, confidence, metadata = {}) {
  if (!Object.values(SignalType).includes(type)) {
    throw new Error(`Invalid signal type: ${type}`);
  }

  if (confidence < 0 || confidence > 1) {
    throw new Error(`Confidence must be between 0 and 1, got: ${confidence}`);
  }

  return {
    signal_type: type,
    observation,
    confidence,
    timestamp: new Date().toISOString(),
    agent_phase: 'phase2',
    agent_layer: 'layer1',
    metadata: {
      ...metadata,
      agent_name: process.env.AGENT_NAME,
      agent_domain: process.env.AGENT_DOMAIN,
    },
    // Explicitly mark as raw observation, not conclusion
    is_conclusion: false,
  };
}

/**
 * Creates an anomaly signal.
 * Anomaly signals indicate unexpected behavior patterns.
 *
 * @param {Object} params
 * @param {string} params.metric - Metric name
 * @param {number} params.observed_value - Observed value
 * @param {number} params.expected_value - Expected value
 * @param {number} params.deviation - Deviation from expected
 * @param {number} params.confidence - Confidence score
 * @returns {Object} Anomaly signal
 */
function createAnomalySignal({ metric, observed_value, expected_value, deviation, confidence }) {
  return createSignal(
    SignalType.ANOMALY,
    {
      metric,
      observed_value,
      expected_value,
      deviation,
      deviation_direction: observed_value > expected_value ? 'above' : 'below',
    },
    confidence,
    { anomaly_type: 'metric_deviation' }
  );
}

/**
 * Creates a drift signal.
 * Drift signals indicate gradual changes in behavior or data patterns.
 *
 * @param {Object} params
 * @param {string} params.dimension - Drift dimension
 * @param {number} params.baseline_value - Baseline value
 * @param {number} params.current_value - Current value
 * @param {number} params.drift_magnitude - Magnitude of drift
 * @param {string} params.window - Time window for drift detection
 * @param {number} params.confidence - Confidence score
 * @returns {Object} Drift signal
 */
function createDriftSignal({ dimension, baseline_value, current_value, drift_magnitude, window, confidence }) {
  return createSignal(
    SignalType.DRIFT,
    {
      dimension,
      baseline_value,
      current_value,
      drift_magnitude,
      drift_direction: current_value > baseline_value ? 'increasing' : 'decreasing',
      window,
    },
    confidence,
    { drift_type: 'behavioral' }
  );
}

/**
 * Creates a memory lineage signal.
 * Lineage signals track memory graph changes (deltas only).
 *
 * IMPORTANT: Emit deltas only, NOT synthesized conclusions.
 *
 * @param {Object} params
 * @param {string} params.operation - Operation type (node_created|edge_created|etc)
 * @param {string} params.node_type - Type of node affected
 * @param {string} params.node_id - Node identifier
 * @param {string} params.parent_id - Parent node (if applicable)
 * @param {Object} params.delta - Change delta
 * @param {number} params.confidence - Confidence score
 * @returns {Object} Memory lineage signal
 */
function createMemoryLineageSignal({ operation, node_type, node_id, parent_id, delta, confidence }) {
  return createSignal(
    SignalType.MEMORY_LINEAGE,
    {
      operation,
      node_type,
      node_id,
      parent_id,
      delta,
      // Lineage chain reference
      lineage_depth: delta?.lineage_depth || 0,
    },
    confidence,
    { lineage_type: 'delta' }
  );
}

/**
 * Creates a latency signal.
 * Latency signals track operation timing.
 *
 * @param {Object} params
 * @param {string} params.operation - Operation name
 * @param {number} params.latency_ms - Latency in milliseconds
 * @param {number} params.budget_ms - Budget in milliseconds
 * @param {boolean} params.within_budget - Whether within budget
 * @param {number} params.confidence - Confidence score
 * @returns {Object} Latency signal
 */
function createLatencySignal({ operation, latency_ms, budget_ms, within_budget, confidence }) {
  return createSignal(
    SignalType.LATENCY,
    {
      operation,
      latency_ms,
      budget_ms,
      within_budget,
      budget_utilization: latency_ms / budget_ms,
    },
    confidence,
    { latency_type: 'operation' }
  );
}

/**
 * Signal emitter for Phase 2 Layer 1.
 * Emits signals to Ruvector as authoritative event index.
 */
class SignalEmitter {
  constructor(ruvectorUrl, apiKey) {
    this.ruvectorUrl = ruvectorUrl;
    this.apiKey = apiKey;
    this.pendingSignals = [];
    this.batchSize = 10;
    this.flushIntervalMs = 1000;
  }

  /**
   * Emit a signal to Ruvector.
   * Signals are batched and flushed periodically.
   *
   * @param {Object} signal - Signal to emit
   */
  async emit(signal) {
    this.pendingSignals.push(signal);

    if (this.pendingSignals.length >= this.batchSize) {
      await this.flush();
    }
  }

  /**
   * Flush pending signals to Ruvector.
   */
  async flush() {
    if (this.pendingSignals.length === 0) return;

    const signals = this.pendingSignals.splice(0);

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);

      await fetch(`${this.ruvectorUrl}/api/v1/signals`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.apiKey}`,
          'X-Agent-Phase': 'phase2',
          'X-Agent-Layer': 'layer1',
        },
        body: JSON.stringify({ signals }),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);
    } catch (error) {
      // Log but don't fail - signals are non-critical
      console.error(`Signal emission failed: ${error.message}`);
      // Re-queue failed signals (up to limit)
      if (this.pendingSignals.length < 100) {
        this.pendingSignals.push(...signals);
      }
    }
  }
}

module.exports = {
  SignalType,
  createSignal,
  createAnomalySignal,
  createDriftSignal,
  createMemoryLineageSignal,
  createLatencySignal,
  SignalEmitter,
};
