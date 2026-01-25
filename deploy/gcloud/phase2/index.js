/**
 * Phase 2 - Operational Intelligence (Layer 1)
 * Main Entry Point
 *
 * Exports all Phase 2 modules for use by agents and service router.
 */

const startup = require('./startup.js');
const signals = require('./signals.js');
const budgets = require('./budgets.js');
const cache = require('./cache.js');

module.exports = {
  // Startup hardening
  validateEnvironment: startup.validateEnvironment,
  verifyRuvector: startup.verifyRuvector,
  initializePhase2: startup.initializePhase2,
  REQUIRED_ENV_VARS: startup.REQUIRED_ENV_VARS,

  // Signal emission
  SignalType: signals.SignalType,
  createSignal: signals.createSignal,
  createAnomalySignal: signals.createAnomalySignal,
  createDriftSignal: signals.createDriftSignal,
  createMemoryLineageSignal: signals.createMemoryLineageSignal,
  createLatencySignal: signals.createLatencySignal,
  SignalEmitter: signals.SignalEmitter,

  // Performance budgets
  DEFAULT_BUDGETS: budgets.DEFAULT_BUDGETS,
  getBudgets: budgets.getBudgets,
  BudgetTracker: budgets.BudgetTracker,
  withBudgetTracking: budgets.withBudgetTracking,

  // Caching
  CACHE_CONFIG: cache.CACHE_CONFIG,
  Phase2Cache: cache.Phase2Cache,
  withHistoricalReadCache: cache.withHistoricalReadCache,
  withLineageLookupCache: cache.withLineageLookupCache,
  getCache: cache.getCache,
};
