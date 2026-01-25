/**
 * Phase 2 - Operational Intelligence (Layer 1)
 * Caching Module
 *
 * CACHING ALLOWED FOR:
 * - Historical reads
 * - Lineage lookups
 *
 * TTL: 60-120 seconds
 */

/**
 * Cache configuration
 */
const CACHE_CONFIG = {
  TTL_MIN_S: 60,
  TTL_MAX_S: 120,
  MAX_ENTRIES: 1000,
  ALLOWED_OPERATIONS: ['historical_read', 'lineage_lookup'],
};

/**
 * Simple in-memory cache with TTL.
 * Used for historical reads and lineage lookups only.
 */
class Phase2Cache {
  constructor(config = {}) {
    this.ttlMinS = config.ttlMinS || CACHE_CONFIG.TTL_MIN_S;
    this.ttlMaxS = config.ttlMaxS || CACHE_CONFIG.TTL_MAX_S;
    this.maxEntries = config.maxEntries || CACHE_CONFIG.MAX_ENTRIES;
    this.cache = new Map();
    this.stats = {
      hits: 0,
      misses: 0,
      evictions: 0,
    };
  }

  /**
   * Generates a random TTL within the configured range.
   * Randomization prevents thundering herd on cache expiry.
   *
   * @returns {number} TTL in milliseconds
   */
  _generateTtl() {
    const ttlS = this.ttlMinS + Math.random() * (this.ttlMaxS - this.ttlMinS);
    return ttlS * 1000;
  }

  /**
   * Creates a cache key from operation type and parameters.
   *
   * @param {string} operation - Operation type
   * @param {Object} params - Operation parameters
   * @returns {string} Cache key
   */
  _makeKey(operation, params) {
    return `${operation}:${JSON.stringify(params)}`;
  }

  /**
   * Validates that the operation is allowed to be cached.
   *
   * @param {string} operation - Operation type
   * @throws {Error} If operation is not cacheable
   */
  _validateOperation(operation) {
    if (!CACHE_CONFIG.ALLOWED_OPERATIONS.includes(operation)) {
      throw new Error(
        `Operation '${operation}' is not cacheable. ` +
        `Allowed operations: ${CACHE_CONFIG.ALLOWED_OPERATIONS.join(', ')}`
      );
    }
  }

  /**
   * Evicts expired entries and enforces max size.
   */
  _cleanup() {
    const now = Date.now();

    // Evict expired entries
    for (const [key, entry] of this.cache.entries()) {
      if (entry.expiresAt < now) {
        this.cache.delete(key);
        this.stats.evictions++;
      }
    }

    // Evict oldest entries if over max size
    if (this.cache.size > this.maxEntries) {
      const entries = Array.from(this.cache.entries())
        .sort((a, b) => a[1].createdAt - b[1].createdAt);

      const toEvict = entries.slice(0, this.cache.size - this.maxEntries);
      for (const [key] of toEvict) {
        this.cache.delete(key);
        this.stats.evictions++;
      }
    }
  }

  /**
   * Gets a value from cache.
   *
   * @param {string} operation - Operation type (must be allowed)
   * @param {Object} params - Operation parameters
   * @returns {*} Cached value or undefined
   */
  get(operation, params) {
    this._validateOperation(operation);

    const key = this._makeKey(operation, params);
    const entry = this.cache.get(key);

    if (!entry) {
      this.stats.misses++;
      return undefined;
    }

    if (entry.expiresAt < Date.now()) {
      this.cache.delete(key);
      this.stats.misses++;
      this.stats.evictions++;
      return undefined;
    }

    this.stats.hits++;
    return entry.value;
  }

  /**
   * Sets a value in cache.
   *
   * @param {string} operation - Operation type (must be allowed)
   * @param {Object} params - Operation parameters
   * @param {*} value - Value to cache
   */
  set(operation, params, value) {
    this._validateOperation(operation);

    const key = this._makeKey(operation, params);
    const now = Date.now();

    this.cache.set(key, {
      value,
      createdAt: now,
      expiresAt: now + this._generateTtl(),
      operation,
    });

    // Cleanup periodically
    if (this.cache.size > this.maxEntries * 1.1) {
      this._cleanup();
    }
  }

  /**
   * Invalidates a cache entry.
   *
   * @param {string} operation - Operation type
   * @param {Object} params - Operation parameters
   */
  invalidate(operation, params) {
    const key = this._makeKey(operation, params);
    this.cache.delete(key);
  }

  /**
   * Clears all cache entries.
   */
  clear() {
    this.cache.clear();
    this.stats.evictions += this.cache.size;
  }

  /**
   * Gets cache statistics.
   *
   * @returns {Object} Cache stats
   */
  getStats() {
    const totalRequests = this.stats.hits + this.stats.misses;
    return {
      size: this.cache.size,
      max_entries: this.maxEntries,
      hits: this.stats.hits,
      misses: this.stats.misses,
      evictions: this.stats.evictions,
      hit_rate: totalRequests > 0 ? this.stats.hits / totalRequests : 0,
      ttl_range_s: [this.ttlMinS, this.ttlMaxS],
    };
  }
}

/**
 * Caching wrapper for historical reads.
 *
 * @param {Function} readFn - Function to wrap
 * @param {Phase2Cache} cache - Cache instance
 * @returns {Function} Cached function
 */
function withHistoricalReadCache(readFn, cache) {
  return async function(params) {
    const cached = cache.get('historical_read', params);
    if (cached !== undefined) {
      return cached;
    }

    const result = await readFn(params);
    cache.set('historical_read', params, result);
    return result;
  };
}

/**
 * Caching wrapper for lineage lookups.
 *
 * @param {Function} lookupFn - Function to wrap
 * @param {Phase2Cache} cache - Cache instance
 * @returns {Function} Cached function
 */
function withLineageLookupCache(lookupFn, cache) {
  return async function(params) {
    const cached = cache.get('lineage_lookup', params);
    if (cached !== undefined) {
      return cached;
    }

    const result = await lookupFn(params);
    cache.set('lineage_lookup', params, result);
    return result;
  };
}

// Singleton cache instance
let cacheInstance = null;

/**
 * Gets or creates the singleton cache instance.
 *
 * @returns {Phase2Cache} Cache instance
 */
function getCache() {
  if (!cacheInstance) {
    cacheInstance = new Phase2Cache();
  }
  return cacheInstance;
}

module.exports = {
  CACHE_CONFIG,
  Phase2Cache,
  withHistoricalReadCache,
  withLineageLookupCache,
  getCache,
};
