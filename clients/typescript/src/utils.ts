/**
 * Utility functions and helper methods
 *
 * Provides convenience functions and utilities for common operations.
 *
 * @module utils
 */

import {
  Node,
  Edge,
  NodeType,
  EdgeType,
  PromptNode,
  ResponseNode,
  ToolInvocationNode,
  TokenUsage,
} from './types';

/**
 * Sleep for a specified duration
 *
 * @param ms - Milliseconds to sleep
 * @returns Promise that resolves after the delay
 *
 * @example
 * ```typescript
 * await sleep(1000); // Wait 1 second
 * console.log('1 second later');
 * ```
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Debounce a function
 *
 * @param fn - Function to debounce
 * @param delayMs - Delay in milliseconds
 * @returns Debounced function
 *
 * @example
 * ```typescript
 * const debouncedSearch = debounce((query: string) => {
 *   console.log('Searching for:', query);
 * }, 300);
 *
 * debouncedSearch('hello'); // Will only execute after 300ms of no calls
 * ```
 */
export function debounce<T extends (...args: any[]) => any>(
  fn: T,
  delayMs: number
): (...args: Parameters<T>) => void {
  let timeoutId: NodeJS.Timeout | null = null;

  return (...args: Parameters<T>) => {
    if (timeoutId) {
      clearTimeout(timeoutId);
    }

    timeoutId = setTimeout(() => {
      fn(...args);
      timeoutId = null;
    }, delayMs);
  };
}

/**
 * Throttle a function
 *
 * @param fn - Function to throttle
 * @param limitMs - Minimum time between calls in milliseconds
 * @returns Throttled function
 *
 * @example
 * ```typescript
 * const throttledUpdate = throttle((value: number) => {
 *   console.log('Update:', value);
 * }, 1000);
 *
 * // Will only execute once per second
 * throttledUpdate(1);
 * throttledUpdate(2);
 * throttledUpdate(3);
 * ```
 */
export function throttle<T extends (...args: any[]) => any>(
  fn: T,
  limitMs: number
): (...args: Parameters<T>) => void {
  let lastCall = 0;

  return (...args: Parameters<T>) => {
    const now = Date.now();
    if (now - lastCall >= limitMs) {
      lastCall = now;
      fn(...args);
    }
  };
}

/**
 * Format timestamp to ISO string
 *
 * @param date - Date to format
 * @returns ISO formatted string
 *
 * @example
 * ```typescript
 * const formatted = formatTimestamp(new Date());
 * // Returns: "2024-01-15T10:30:00.000Z"
 * ```
 */
export function formatTimestamp(date: Date): string {
  return date.toISOString();
}

/**
 * Parse ISO timestamp string to Date
 *
 * @param timestamp - ISO timestamp string
 * @returns Date object
 *
 * @example
 * ```typescript
 * const date = parseTimestamp("2024-01-15T10:30:00.000Z");
 * ```
 */
export function parseTimestamp(timestamp: string): Date {
  return new Date(timestamp);
}

/**
 * Calculate time difference in milliseconds
 *
 * @param start - Start date
 * @param end - End date (defaults to now)
 * @returns Difference in milliseconds
 *
 * @example
 * ```typescript
 * const start = new Date();
 * await someOperation();
 * const duration = timeDiff(start);
 * console.log(`Operation took ${duration}ms`);
 * ```
 */
export function timeDiff(start: Date, end: Date = new Date()): number {
  return end.getTime() - start.getTime();
}

/**
 * Format duration in milliseconds to human-readable string
 *
 * @param ms - Duration in milliseconds
 * @returns Formatted string
 *
 * @example
 * ```typescript
 * formatDuration(1500); // "1.5s"
 * formatDuration(65000); // "1m 5s"
 * formatDuration(3661000); // "1h 1m 1s"
 * ```
 */
export function formatDuration(ms: number): string {
  if (ms < 1000) {
    return `${ms}ms`;
  }

  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) {
    return `${seconds}s`;
  }

  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes < 60) {
    return remainingSeconds > 0 ? `${minutes}m ${remainingSeconds}s` : `${minutes}m`;
  }

  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return remainingSeconds > 0
    ? `${hours}h ${remainingMinutes}m ${remainingSeconds}s`
    : `${hours}h ${remainingMinutes}m`;
}

/**
 * Calculate token usage statistics
 *
 * @param tokenUsages - Array of token usage objects
 * @returns Aggregated token usage
 *
 * @example
 * ```typescript
 * const total = calculateTokenUsage([
 *   { promptTokens: 10, completionTokens: 20, totalTokens: 30 },
 *   { promptTokens: 15, completionTokens: 25, totalTokens: 40 }
 * ]);
 * // Returns: { promptTokens: 25, completionTokens: 45, totalTokens: 70 }
 * ```
 */
export function calculateTokenUsage(tokenUsages: TokenUsage[]): TokenUsage {
  return tokenUsages.reduce(
    (acc, usage) => ({
      promptTokens: acc.promptTokens + usage.promptTokens,
      completionTokens: acc.completionTokens + usage.completionTokens,
      totalTokens: acc.totalTokens + usage.totalTokens,
    }),
    { promptTokens: 0, completionTokens: 0, totalTokens: 0 }
  );
}

/**
 * Type guard for PromptNode
 *
 * @param node - Node to check
 * @returns True if node is a PromptNode
 *
 * @example
 * ```typescript
 * if (isPromptNode(node)) {
 *   console.log('Prompt content:', node.data.content);
 * }
 * ```
 */
export function isPromptNode(node: Node): node is Node & { data: PromptNode } {
  return node.type === NodeType.PROMPT && !!node.data;
}

/**
 * Type guard for ResponseNode
 *
 * @param node - Node to check
 * @returns True if node is a ResponseNode
 *
 * @example
 * ```typescript
 * if (isResponseNode(node)) {
 *   console.log('Response content:', node.data.content);
 * }
 * ```
 */
export function isResponseNode(node: Node): node is Node & { data: ResponseNode } {
  return node.type === NodeType.RESPONSE && !!node.data;
}

/**
 * Type guard for ToolInvocationNode
 *
 * @param node - Node to check
 * @returns True if node is a ToolInvocationNode
 *
 * @example
 * ```typescript
 * if (isToolInvocationNode(node)) {
 *   console.log('Tool name:', node.data.toolName);
 * }
 * ```
 */
export function isToolInvocationNode(node: Node): node is Node & { data: ToolInvocationNode } {
  return node.type === NodeType.TOOL_INVOCATION && !!node.data;
}

/**
 * Filter nodes by type
 *
 * @param nodes - Nodes to filter
 * @param type - Node type to filter by
 * @returns Filtered nodes
 *
 * @example
 * ```typescript
 * const prompts = filterNodesByType(nodes, NodeType.PROMPT);
 * ```
 */
export function filterNodesByType(nodes: Node[], type: NodeType): Node[] {
  return nodes.filter((node) => node.type === type);
}

/**
 * Filter edges by type
 *
 * @param edges - Edges to filter
 * @param type - Edge type to filter by
 * @returns Filtered edges
 *
 * @example
 * ```typescript
 * const responses = filterEdgesByType(edges, EdgeType.RESPONDS_TO);
 * ```
 */
export function filterEdgesByType(edges: Edge[], type: EdgeType): Edge[] {
  return edges.filter((edge) => edge.type === type);
}

/**
 * Group nodes by session ID
 *
 * @param nodes - Nodes to group
 * @returns Map of session ID to nodes
 *
 * @example
 * ```typescript
 * const grouped = groupNodesBySession(nodes);
 * for (const [sessionId, sessionNodes] of grouped.entries()) {
 *   console.log(`Session ${sessionId}: ${sessionNodes.length} nodes`);
 * }
 * ```
 */
export function groupNodesBySession(nodes: Node[]): Map<string, Node[]> {
  const grouped = new Map<string, Node[]>();

  for (const node of nodes) {
    if (isPromptNode(node)) {
      const sessionId = node.data.sessionId;
      if (!grouped.has(sessionId)) {
        grouped.set(sessionId, []);
      }
      grouped.get(sessionId)!.push(node);
    }
  }

  return grouped;
}

/**
 * Sort nodes by creation time
 *
 * @param nodes - Nodes to sort
 * @param ascending - Sort in ascending order (default: true)
 * @returns Sorted nodes
 *
 * @example
 * ```typescript
 * const sorted = sortNodesByTime(nodes, false); // Newest first
 * ```
 */
export function sortNodesByTime(nodes: Node[], ascending = true): Node[] {
  return [...nodes].sort((a, b) => {
    const timeA = a.createdAt.getTime();
    const timeB = b.createdAt.getTime();
    return ascending ? timeA - timeB : timeB - timeA;
  });
}

/**
 * Sort edges by creation time
 *
 * @param edges - Edges to sort
 * @param ascending - Sort in ascending order (default: true)
 * @returns Sorted edges
 *
 * @example
 * ```typescript
 * const sorted = sortEdgesByTime(edges, false); // Newest first
 * ```
 */
export function sortEdgesByTime(edges: Edge[], ascending = true): Edge[] {
  return [...edges].sort((a, b) => {
    const timeA = a.createdAt.getTime();
    const timeB = b.createdAt.getTime();
    return ascending ? timeA - timeB : timeB - timeA;
  });
}

/**
 * Chunk array into smaller arrays
 *
 * @param array - Array to chunk
 * @param size - Size of each chunk
 * @returns Array of chunks
 *
 * @example
 * ```typescript
 * const chunks = chunk([1, 2, 3, 4, 5], 2);
 * // Returns: [[1, 2], [3, 4], [5]]
 * ```
 */
export function chunk<T>(array: T[], size: number): T[][] {
  const chunks: T[][] = [];
  for (let i = 0; i < array.length; i += size) {
    chunks.push(array.slice(i, i + size));
  }
  return chunks;
}

/**
 * Batch process items with rate limiting
 *
 * @param items - Items to process
 * @param processFn - Function to process each item
 * @param batchSize - Number of items to process in parallel
 * @param delayMs - Delay between batches in milliseconds
 * @returns Promise with array of results
 *
 * @example
 * ```typescript
 * const results = await batchProcess(
 *   nodeIds,
 *   (id) => client.getNode(id),
 *   10, // Process 10 at a time
 *   100 // Wait 100ms between batches
 * );
 * ```
 */
export async function batchProcess<T, R>(
  items: T[],
  processFn: (item: T) => Promise<R>,
  batchSize: number,
  delayMs = 0
): Promise<R[]> {
  const results: R[] = [];
  const chunks = chunk(items, batchSize);

  for (let i = 0; i < chunks.length; i++) {
    const batchResults = await Promise.all(chunks[i].map(processFn));
    results.push(...batchResults);

    // Add delay between batches (except for the last one)
    if (i < chunks.length - 1 && delayMs > 0) {
      await sleep(delayMs);
    }
  }

  return results;
}

/**
 * Retry a promise with exponential backoff
 *
 * @param fn - Function that returns a promise
 * @param maxRetries - Maximum number of retries
 * @param initialDelayMs - Initial delay in milliseconds
 * @returns Promise with the result
 * @deprecated Use withRetry from retry module instead
 */
export async function retryWithBackoff<T>(
  fn: () => Promise<T>,
  maxRetries = 3,
  initialDelayMs = 100
): Promise<T> {
  let lastError: Error | undefined;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error as Error;

      if (attempt < maxRetries) {
        const delayMs = initialDelayMs * Math.pow(2, attempt);
        await sleep(delayMs);
      }
    }
  }

  throw lastError;
}

/**
 * Create a promise with timeout
 *
 * @param promise - Promise to wrap
 * @param timeoutMs - Timeout in milliseconds
 * @param errorMessage - Error message if timeout occurs
 * @returns Promise that rejects on timeout
 *
 * @example
 * ```typescript
 * const result = await withTimeout(
 *   client.getNode('node-id'),
 *   5000,
 *   'Operation timed out after 5 seconds'
 * );
 * ```
 */
export function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  errorMessage = 'Operation timed out'
): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) => setTimeout(() => reject(new Error(errorMessage)), timeoutMs)),
  ]);
}

/**
 * Deep clone an object
 *
 * @param obj - Object to clone
 * @returns Cloned object
 *
 * @example
 * ```typescript
 * const copy = deepClone(originalObject);
 * ```
 */
export function deepClone<T>(obj: T): T {
  return JSON.parse(JSON.stringify(obj));
}

/**
 * Merge objects deeply
 *
 * @param target - Target object
 * @param sources - Source objects to merge
 * @returns Merged object
 *
 * @example
 * ```typescript
 * const merged = deepMerge({ a: 1 }, { b: 2 }, { c: 3 });
 * // Returns: { a: 1, b: 2, c: 3 }
 * ```
 */
export function deepMerge<T extends Record<string, any>>(target: T, ...sources: Partial<T>[]): T {
  if (!sources.length) return target;

  const source = sources.shift();
  if (!source) return target;

  for (const key in source) {
    if (Object.prototype.hasOwnProperty.call(source, key)) {
      const value = source[key];
      if (value && typeof value === 'object' && !Array.isArray(value)) {
        if (!target[key]) {
          target[key] = {} as any;
        }
        deepMerge(target[key], value);
      } else {
        target[key] = value as any;
      }
    }
  }

  return deepMerge(target, ...sources);
}

/**
 * Convert object to query string
 *
 * @param params - Object to convert
 * @returns Query string
 *
 * @example
 * ```typescript
 * const qs = toQueryString({ limit: 10, offset: 0 });
 * // Returns: "limit=10&offset=0"
 * ```
 */
export function toQueryString(params: Record<string, any>): string {
  return Object.entries(params)
    .filter(([_, value]) => value !== undefined && value !== null)
    .map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(String(value))}`)
    .join('&');
}

/**
 * Generate a unique ID
 *
 * @returns Unique ID string
 *
 * @example
 * ```typescript
 * const id = generateId();
 * // Returns: "abc123def456"
 * ```
 */
export function generateId(): string {
  return Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15);
}

/**
 * Safe JSON parse with fallback
 *
 * @param json - JSON string to parse
 * @param fallback - Fallback value if parsing fails
 * @returns Parsed object or fallback
 *
 * @example
 * ```typescript
 * const data = safeJsonParse('{"key": "value"}', {});
 * const invalid = safeJsonParse('invalid json', { default: true });
 * ```
 */
export function safeJsonParse<T>(json: string, fallback: T): T {
  try {
    return JSON.parse(json);
  } catch {
    return fallback;
  }
}

/**
 * Safe JSON stringify
 *
 * @param obj - Object to stringify
 * @param fallback - Fallback string if stringify fails
 * @returns JSON string or fallback
 *
 * @example
 * ```typescript
 * const json = safeJsonStringify({ key: 'value' }, '{}');
 * ```
 */
export function safeJsonStringify(obj: any, fallback = '{}'): string {
  try {
    return JSON.stringify(obj);
  } catch {
    return fallback;
  }
}

/**
 * Check if value is empty (null, undefined, empty string, empty array, empty object)
 *
 * @param value - Value to check
 * @returns True if value is empty
 *
 * @example
 * ```typescript
 * isEmpty(null); // true
 * isEmpty(''); // true
 * isEmpty([]); // true
 * isEmpty({}); // true
 * isEmpty('hello'); // false
 * ```
 */
export function isEmpty(value: any): boolean {
  if (value === null || value === undefined) return true;
  if (typeof value === 'string') return value.trim().length === 0;
  if (Array.isArray(value)) return value.length === 0;
  if (typeof value === 'object') return Object.keys(value).length === 0;
  return false;
}

/**
 * Omit properties from object
 *
 * @param obj - Source object
 * @param keys - Keys to omit
 * @returns New object without specified keys
 *
 * @example
 * ```typescript
 * const obj = { a: 1, b: 2, c: 3 };
 * const result = omit(obj, ['b', 'c']);
 * // Returns: { a: 1 }
 * ```
 */
export function omit<T extends Record<string, any>, K extends keyof T>(
  obj: T,
  keys: K[]
): Omit<T, K> {
  const result = { ...obj };
  for (const key of keys) {
    delete result[key];
  }
  return result;
}

/**
 * Pick properties from object
 *
 * @param obj - Source object
 * @param keys - Keys to pick
 * @returns New object with only specified keys
 *
 * @example
 * ```typescript
 * const obj = { a: 1, b: 2, c: 3 };
 * const result = pick(obj, ['a', 'c']);
 * // Returns: { a: 1, c: 3 }
 * ```
 */
export function pick<T extends Record<string, any>, K extends keyof T>(
  obj: T,
  keys: K[]
): Pick<T, K> {
  const result = {} as Pick<T, K>;
  for (const key of keys) {
    if (key in obj) {
      result[key] = obj[key];
    }
  }
  return result;
}
