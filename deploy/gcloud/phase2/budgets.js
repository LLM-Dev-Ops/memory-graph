/**
 * Phase 2 - Operational Intelligence (Layer 1)
 * Performance Budget Module
 *
 * PERFORMANCE BUDGETS:
 * - MAX_TOKENS=1000
 * - MAX_LATENCY_MS=2000
 * - MAX_CALLS_PER_RUN=3
 */

const { createLatencySignal, createAnomalySignal } = require('./signals.js');

/**
 * Default performance budgets
 */
const DEFAULT_BUDGETS = {
  MAX_TOKENS: 1000,
  MAX_LATENCY_MS: 2000,
  MAX_CALLS_PER_RUN: 3,
};

/**
 * Gets current budget configuration from environment or defaults.
 * @returns {Object} Budget configuration
 */
function getBudgets() {
  return {
    MAX_TOKENS: parseInt(process.env.MAX_TOKENS || String(DEFAULT_BUDGETS.MAX_TOKENS), 10),
    MAX_LATENCY_MS: parseInt(process.env.MAX_LATENCY_MS || String(DEFAULT_BUDGETS.MAX_LATENCY_MS), 10),
    MAX_CALLS_PER_RUN: parseInt(process.env.MAX_CALLS_PER_RUN || String(DEFAULT_BUDGETS.MAX_CALLS_PER_RUN), 10),
  };
}

/**
 * Budget tracker for a single agent run.
 * Enforces performance budgets and emits signals on violations.
 */
class BudgetTracker {
  constructor(signalEmitter) {
    this.budgets = getBudgets();
    this.signalEmitter = signalEmitter;
    this.runId = crypto.randomUUID();
    this.startTime = Date.now();

    // Tracking state
    this.tokensUsed = 0;
    this.callsMade = 0;
    this.latencyMs = 0;
    this.violations = [];
  }

  /**
   * Records token usage and checks against budget.
   *
   * @param {number} tokens - Tokens used
   * @returns {boolean} True if within budget
   */
  recordTokens(tokens) {
    this.tokensUsed += tokens;

    if (this.tokensUsed > this.budgets.MAX_TOKENS) {
      this.violations.push({
        type: 'token_budget_exceeded',
        budget: this.budgets.MAX_TOKENS,
        actual: this.tokensUsed,
        timestamp: new Date().toISOString(),
      });

      if (this.signalEmitter) {
        this.signalEmitter.emit(createAnomalySignal({
          metric: 'token_usage',
          observed_value: this.tokensUsed,
          expected_value: this.budgets.MAX_TOKENS,
          deviation: this.tokensUsed - this.budgets.MAX_TOKENS,
          confidence: 1.0,
        }));
      }

      return false;
    }
    return true;
  }

  /**
   * Records an API call and checks against budget.
   *
   * @returns {boolean} True if within budget
   */
  recordCall() {
    this.callsMade++;

    if (this.callsMade > this.budgets.MAX_CALLS_PER_RUN) {
      this.violations.push({
        type: 'calls_budget_exceeded',
        budget: this.budgets.MAX_CALLS_PER_RUN,
        actual: this.callsMade,
        timestamp: new Date().toISOString(),
      });

      if (this.signalEmitter) {
        this.signalEmitter.emit(createAnomalySignal({
          metric: 'api_calls',
          observed_value: this.callsMade,
          expected_value: this.budgets.MAX_CALLS_PER_RUN,
          deviation: this.callsMade - this.budgets.MAX_CALLS_PER_RUN,
          confidence: 1.0,
        }));
      }

      return false;
    }
    return true;
  }

  /**
   * Checks if more calls are allowed.
   *
   * @returns {boolean} True if more calls allowed
   */
  canMakeCall() {
    return this.callsMade < this.budgets.MAX_CALLS_PER_RUN;
  }

  /**
   * Gets remaining budget for calls.
   *
   * @returns {number} Remaining calls allowed
   */
  getRemainingCalls() {
    return Math.max(0, this.budgets.MAX_CALLS_PER_RUN - this.callsMade);
  }

  /**
   * Records operation latency and checks against budget.
   *
   * @param {number} latencyMs - Latency in milliseconds
   * @param {string} operation - Operation name
   * @returns {boolean} True if within budget
   */
  recordLatency(latencyMs, operation = 'unknown') {
    this.latencyMs = Date.now() - this.startTime;

    const withinBudget = latencyMs <= this.budgets.MAX_LATENCY_MS;

    if (this.signalEmitter) {
      this.signalEmitter.emit(createLatencySignal({
        operation,
        latency_ms: latencyMs,
        budget_ms: this.budgets.MAX_LATENCY_MS,
        within_budget: withinBudget,
        confidence: 1.0,
      }));
    }

    if (!withinBudget) {
      this.violations.push({
        type: 'latency_budget_exceeded',
        budget: this.budgets.MAX_LATENCY_MS,
        actual: latencyMs,
        operation,
        timestamp: new Date().toISOString(),
      });
    }

    return withinBudget;
  }

  /**
   * Gets elapsed time since run start.
   *
   * @returns {number} Elapsed milliseconds
   */
  getElapsedMs() {
    return Date.now() - this.startTime;
  }

  /**
   * Checks if latency budget is likely to be exceeded.
   *
   * @param {number} estimatedRemainingMs - Estimated remaining time
   * @returns {boolean} True if likely to exceed
   */
  willExceedLatency(estimatedRemainingMs) {
    return (this.getElapsedMs() + estimatedRemainingMs) > this.budgets.MAX_LATENCY_MS;
  }

  /**
   * Checks if any budget has been violated.
   *
   * @returns {boolean} True if any violation occurred
   */
  hasViolations() {
    return this.violations.length > 0;
  }

  /**
   * Gets budget usage summary.
   *
   * @returns {Object} Budget summary
   */
  getSummary() {
    const totalLatency = Date.now() - this.startTime;

    return {
      run_id: this.runId,
      tokens: {
        used: this.tokensUsed,
        budget: this.budgets.MAX_TOKENS,
        utilization: this.tokensUsed / this.budgets.MAX_TOKENS,
        within_budget: this.tokensUsed <= this.budgets.MAX_TOKENS,
      },
      calls: {
        made: this.callsMade,
        budget: this.budgets.MAX_CALLS_PER_RUN,
        utilization: this.callsMade / this.budgets.MAX_CALLS_PER_RUN,
        within_budget: this.callsMade <= this.budgets.MAX_CALLS_PER_RUN,
      },
      latency: {
        total_ms: totalLatency,
        budget_ms: this.budgets.MAX_LATENCY_MS,
        utilization: totalLatency / this.budgets.MAX_LATENCY_MS,
        within_budget: totalLatency <= this.budgets.MAX_LATENCY_MS,
      },
      violations: this.violations,
      all_budgets_met: !this.hasViolations() && totalLatency <= this.budgets.MAX_LATENCY_MS,
    };
  }
}

/**
 * Decorator that wraps a function with budget tracking.
 *
 * @param {Function} fn - Function to wrap
 * @param {BudgetTracker} tracker - Budget tracker instance
 * @param {string} operation - Operation name for latency tracking
 * @returns {Function} Wrapped function
 */
function withBudgetTracking(fn, tracker, operation) {
  return async function(...args) {
    if (!tracker.canMakeCall()) {
      throw new Error(`Budget exceeded: maximum ${tracker.budgets.MAX_CALLS_PER_RUN} calls allowed`);
    }

    tracker.recordCall();
    const startTime = Date.now();

    try {
      const result = await fn.apply(this, args);
      tracker.recordLatency(Date.now() - startTime, operation);
      return result;
    } catch (error) {
      tracker.recordLatency(Date.now() - startTime, `${operation}_error`);
      throw error;
    }
  };
}

module.exports = {
  DEFAULT_BUDGETS,
  getBudgets,
  BudgetTracker,
  withBudgetTracking,
};
