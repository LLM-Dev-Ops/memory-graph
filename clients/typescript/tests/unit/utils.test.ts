/**
 * Unit tests for utils module
 */

import { describe, it, expect, jest, beforeEach, afterEach } from '@jest/globals';
import { NodeType, EdgeType } from '../../src/types';
import {
  sleep,
  debounce,
  throttle,
  formatTimestamp,
  parseTimestamp,
  timeDiff,
  formatDuration,
  calculateTokenUsage,
  isPromptNode,
  isResponseNode,
  isToolInvocationNode,
  filterNodesByType,
  filterEdgesByType,
  groupNodesBySession,
  sortNodesByTime,
  sortEdgesByTime,
  chunk,
  batchProcess,
  retryWithBackoff,
  withTimeout,
  deepClone,
  deepMerge,
  toQueryString,
  generateId,
  safeJsonParse,
  safeJsonStringify,
  isEmpty,
  omit,
  pick,
} from '../../src/utils';
import { generateMockNode, generateMockEdge, mockPromptNodeData } from '../fixtures/mock-data';

describe('sleep', () => {
  it('should wait for specified time', async () => {
    jest.useFakeTimers();
    const promise = sleep(1000);
    jest.advanceTimersByTime(1000);
    await promise;
    jest.useRealTimers();
  });
});

describe('debounce', () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('should debounce function calls', () => {
    const fn = jest.fn<(...args: any[]) => any>();
    const debounced = debounce(fn, 300);

    debounced('arg1');
    debounced('arg2');
    debounced('arg3');

    expect(fn).not.toHaveBeenCalled();

    jest.advanceTimersByTime(300);

    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith('arg3');
  });

  it('should reset timer on new calls', () => {
    const fn = jest.fn<(...args: any[]) => any>();
    const debounced = debounce(fn, 300);

    debounced('arg1');
    jest.advanceTimersByTime(200);
    debounced('arg2');
    jest.advanceTimersByTime(200);

    expect(fn).not.toHaveBeenCalled();

    jest.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledTimes(1);
  });
});

describe('throttle', () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('should throttle function calls', () => {
    const fn = jest.fn<(...args: any[]) => any>();
    const throttled = throttle(fn, 1000);

    throttled('arg1');
    throttled('arg2');
    throttled('arg3');

    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith('arg1');

    jest.advanceTimersByTime(1000);
    throttled('arg4');

    expect(fn).toHaveBeenCalledTimes(2);
    expect(fn).toHaveBeenCalledWith('arg4');
  });
});

describe('Time utilities', () => {
  describe('formatTimestamp', () => {
    it('should format date to ISO string', () => {
      const date = new Date('2024-01-15T10:30:00.000Z');
      const result = formatTimestamp(date);
      expect(result).toBe('2024-01-15T10:30:00.000Z');
    });
  });

  describe('parseTimestamp', () => {
    it('should parse ISO string to Date', () => {
      const result = parseTimestamp('2024-01-15T10:30:00.000Z');
      expect(result).toBeInstanceOf(Date);
      expect(result.toISOString()).toBe('2024-01-15T10:30:00.000Z');
    });
  });

  describe('timeDiff', () => {
    it('should calculate time difference', () => {
      const start = new Date('2024-01-15T10:00:00.000Z');
      const end = new Date('2024-01-15T10:05:00.000Z');
      const diff = timeDiff(start, end);
      expect(diff).toBe(5 * 60 * 1000); // 5 minutes in ms
    });

    it('should use current time if end not provided', () => {
      const start = new Date(Date.now() - 1000);
      const diff = timeDiff(start);
      expect(diff).toBeGreaterThanOrEqual(1000);
      expect(diff).toBeLessThan(2000);
    });
  });

  describe('formatDuration', () => {
    it('should format milliseconds', () => {
      expect(formatDuration(500)).toBe('500ms');
      expect(formatDuration(999)).toBe('999ms');
    });

    it('should format seconds', () => {
      expect(formatDuration(1000)).toBe('1s');
      expect(formatDuration(45000)).toBe('45s');
    });

    it('should format minutes', () => {
      expect(formatDuration(60000)).toBe('1m');
      expect(formatDuration(65000)).toBe('1m 5s');
      expect(formatDuration(180000)).toBe('3m');
    });

    it('should format hours', () => {
      expect(formatDuration(3600000)).toBe('1h 0m');
      expect(formatDuration(3661000)).toBe('1h 1m 1s');
      expect(formatDuration(7200000)).toBe('2h 0m');
    });
  });
});

describe('calculateTokenUsage', () => {
  it('should aggregate token usage', () => {
    const usages = [
      { promptTokens: 10, completionTokens: 20, totalTokens: 30 },
      { promptTokens: 15, completionTokens: 25, totalTokens: 40 },
      { promptTokens: 5, completionTokens: 10, totalTokens: 15 },
    ];

    const result = calculateTokenUsage(usages);

    expect(result).toEqual({
      promptTokens: 30,
      completionTokens: 55,
      totalTokens: 85,
    });
  });

  it('should handle empty array', () => {
    const result = calculateTokenUsage([]);
    expect(result).toEqual({
      promptTokens: 0,
      completionTokens: 0,
      totalTokens: 0,
    });
  });
});

describe('Type guards', () => {
  describe('isPromptNode', () => {
    it('should return true for prompt nodes', () => {
      const node = generateMockNode(NodeType.PROMPT);
      expect(isPromptNode(node)).toBe(true);
    });

    it('should return false for non-prompt nodes', () => {
      const node = generateMockNode(NodeType.RESPONSE);
      expect(isPromptNode(node)).toBe(false);
    });
  });

  describe('isResponseNode', () => {
    it('should return true for response nodes', () => {
      const node = generateMockNode(NodeType.RESPONSE);
      expect(isResponseNode(node)).toBe(true);
    });

    it('should return false for non-response nodes', () => {
      const node = generateMockNode(NodeType.PROMPT);
      expect(isResponseNode(node)).toBe(false);
    });
  });

  describe('isToolInvocationNode', () => {
    it('should return true for tool invocation nodes', () => {
      const node = generateMockNode(NodeType.TOOL_INVOCATION);
      expect(isToolInvocationNode(node)).toBe(true);
    });

    it('should return false for non-tool nodes', () => {
      const node = generateMockNode(NodeType.PROMPT);
      expect(isToolInvocationNode(node)).toBe(false);
    });
  });
});

describe('Filter functions', () => {
  describe('filterNodesByType', () => {
    it('should filter nodes by type', () => {
      const nodes = [
        generateMockNode(NodeType.PROMPT),
        generateMockNode(NodeType.RESPONSE),
        generateMockNode(NodeType.PROMPT),
      ];

      const prompts = filterNodesByType(nodes, NodeType.PROMPT);
      expect(prompts).toHaveLength(2);
      expect(prompts.every((n) => n.type === NodeType.PROMPT)).toBe(true);
    });
  });

  describe('filterEdgesByType', () => {
    it('should filter edges by type', () => {
      const edges = [
        generateMockEdge({ type: EdgeType.RESPONDS_TO }),
        generateMockEdge({ type: EdgeType.INVOKES }),
        generateMockEdge({ type: EdgeType.RESPONDS_TO }),
      ];

      const responses = filterEdgesByType(edges, EdgeType.RESPONDS_TO);
      expect(responses).toHaveLength(2);
      expect(responses.every((e) => e.type === EdgeType.RESPONDS_TO)).toBe(true);
    });
  });
});

describe('groupNodesBySession', () => {
  it('should group nodes by session ID', () => {
    const nodes = [
      { ...generateMockNode(NodeType.PROMPT), data: { ...mockPromptNodeData, sessionId: 'session-1' } },
      { ...generateMockNode(NodeType.PROMPT), data: { ...mockPromptNodeData, sessionId: 'session-2' } },
      { ...generateMockNode(NodeType.PROMPT), data: { ...mockPromptNodeData, sessionId: 'session-1' } },
    ];

    const grouped = groupNodesBySession(nodes);

    expect(grouped.size).toBe(2);
    expect(grouped.get('session-1')).toHaveLength(2);
    expect(grouped.get('session-2')).toHaveLength(1);
  });

  it('should skip non-prompt nodes', () => {
    const nodes = [
      { ...generateMockNode(NodeType.PROMPT), data: { ...mockPromptNodeData, sessionId: 'session-1' } },
      generateMockNode(NodeType.RESPONSE),
    ];

    const grouped = groupNodesBySession(nodes);

    expect(grouped.size).toBe(1);
    expect(grouped.get('session-1')).toHaveLength(1);
  });
});

describe('Sort functions', () => {
  describe('sortNodesByTime', () => {
    it('should sort nodes ascending by default', () => {
      const nodes = [
        generateMockNode(NodeType.PROMPT, { createdAt: new Date('2024-01-03') }),
        generateMockNode(NodeType.PROMPT, { createdAt: new Date('2024-01-01') }),
        generateMockNode(NodeType.PROMPT, { createdAt: new Date('2024-01-02') }),
      ];

      const sorted = sortNodesByTime(nodes);

      expect(sorted[0].createdAt.getTime()).toBeLessThan(sorted[1].createdAt.getTime());
      expect(sorted[1].createdAt.getTime()).toBeLessThan(sorted[2].createdAt.getTime());
    });

    it('should sort nodes descending when specified', () => {
      const nodes = [
        generateMockNode(NodeType.PROMPT, { createdAt: new Date('2024-01-01') }),
        generateMockNode(NodeType.PROMPT, { createdAt: new Date('2024-01-03') }),
        generateMockNode(NodeType.PROMPT, { createdAt: new Date('2024-01-02') }),
      ];

      const sorted = sortNodesByTime(nodes, false);

      expect(sorted[0].createdAt.getTime()).toBeGreaterThan(sorted[1].createdAt.getTime());
      expect(sorted[1].createdAt.getTime()).toBeGreaterThan(sorted[2].createdAt.getTime());
    });
  });

  describe('sortEdgesByTime', () => {
    it('should sort edges by time', () => {
      const edges = [
        generateMockEdge({ createdAt: new Date('2024-01-03') }),
        generateMockEdge({ createdAt: new Date('2024-01-01') }),
        generateMockEdge({ createdAt: new Date('2024-01-02') }),
      ];

      const sorted = sortEdgesByTime(edges);

      expect(sorted[0].createdAt.getTime()).toBeLessThan(sorted[1].createdAt.getTime());
      expect(sorted[1].createdAt.getTime()).toBeLessThan(sorted[2].createdAt.getTime());
    });
  });
});

describe('chunk', () => {
  it('should chunk array into smaller arrays', () => {
    const array = [1, 2, 3, 4, 5, 6, 7, 8];
    const chunks = chunk(array, 3);

    expect(chunks).toEqual([[1, 2, 3], [4, 5, 6], [7, 8]]);
  });

  it('should handle array smaller than chunk size', () => {
    const array = [1, 2];
    const chunks = chunk(array, 5);

    expect(chunks).toEqual([[1, 2]]);
  });

  it('should handle empty array', () => {
    const chunks = chunk([], 3);
    expect(chunks).toEqual([]);
  });
});

describe('batchProcess', () => {
  it('should process items in batches', async () => {
    const items = [1, 2, 3, 4, 5];
    const processFn = jest.fn<(n: number) => Promise<number>>().mockImplementation((n) => Promise.resolve(n * 2));

    const results = await batchProcess(items, processFn, 2, 0);

    expect(results).toEqual([2, 4, 6, 8, 10]);
    expect(processFn).toHaveBeenCalledTimes(5);
  });

  it('should delay between batches', async () => {
    jest.useFakeTimers();

    const items = [1, 2, 3, 4];
    const processFn = jest.fn<(n: number) => Promise<number>>().mockImplementation((n) => Promise.resolve(n * 2));

    const promise = batchProcess(items, processFn, 2, 100);

    await jest.advanceTimersByTimeAsync(0);
    await jest.advanceTimersByTimeAsync(100);
    await jest.advanceTimersByTimeAsync(0);

    const results = await promise;

    expect(results).toEqual([2, 4, 6, 8]);

    jest.useRealTimers();
  });
});

describe('retryWithBackoff', () => {
  it('should succeed on first attempt', async () => {
    const fn = jest.fn<() => Promise<string>>().mockResolvedValue('success');
    const result = await retryWithBackoff(fn, 3, 10);

    expect(result).toBe('success');
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it('should retry on failure', async () => {
    jest.useFakeTimers();

    const fn = jest.fn<() => Promise<string>>().mockRejectedValueOnce(new Error('Failed')).mockResolvedValue('success');

    const promise = retryWithBackoff(fn, 3, 10);

    await jest.advanceTimersByTimeAsync(10);

    const result = await promise;

    expect(result).toBe('success');
    expect(fn).toHaveBeenCalledTimes(2);

    jest.useRealTimers();
  });
});

describe('withTimeout', () => {
  it('should resolve if promise completes in time', async () => {
    const promise = Promise.resolve('success');
    const result = await withTimeout(promise, 1000);

    expect(result).toBe('success');
  });

  it('should reject on timeout', async () => {
    jest.useFakeTimers();

    const promise = new Promise((resolve) => setTimeout(resolve, 2000));
    const timeoutPromise = withTimeout(promise, 1000, 'Timeout error');

    jest.advanceTimersByTime(1001);

    await expect(timeoutPromise).rejects.toThrow('Timeout error');

    jest.useRealTimers();
  });
});

describe('deepClone', () => {
  it('should deep clone object', () => {
    const obj = { a: 1, b: { c: 2, d: [3, 4] } };
    const clone = deepClone(obj);

    expect(clone).toEqual(obj);
    expect(clone).not.toBe(obj);
    expect(clone.b).not.toBe(obj.b);
  });
});

describe('deepMerge', () => {
  it('should merge objects deeply', () => {
    const obj1: { a: number; b: { c: number; d?: number }; e?: number } = { a: 1, b: { c: 2 } };
    const obj2: Partial<typeof obj1> = { b: { c: 2, d: 3 }, e: 4 };

    const result = deepMerge(obj1, obj2);

    expect(result).toEqual({ a: 1, b: { c: 2, d: 3 }, e: 4 });
  });

  it('should handle multiple sources', () => {
    const obj1: { a: number; b?: number; c?: number } = { a: 1 };
    const obj2: Partial<typeof obj1> = { b: 2 };
    const obj3: Partial<typeof obj1> = { c: 3 };

    const result = deepMerge(obj1, obj2, obj3);

    expect(result).toEqual({ a: 1, b: 2, c: 3 });
  });

  it('should overwrite arrays', () => {
    const obj1 = { arr: [1, 2] };
    const obj2 = { arr: [3, 4] };

    const result = deepMerge(obj1, obj2);

    expect(result.arr).toEqual([3, 4]);
  });
});

describe('toQueryString', () => {
  it('should convert object to query string', () => {
    const params = { limit: 10, offset: 0, filter: 'test' };
    const result = toQueryString(params);

    expect(result).toBe('limit=10&offset=0&filter=test');
  });

  it('should skip undefined and null values', () => {
    const params = { a: 'value', b: undefined, c: null, d: 'test' };
    const result = toQueryString(params);

    expect(result).toBe('a=value&d=test');
  });

  it('should encode special characters', () => {
    const params = { key: 'value with spaces', special: 'a&b=c' };
    const result = toQueryString(params);

    expect(result).toContain('value%20with%20spaces');
    expect(result).toContain('a%26b%3Dc');
  });
});

describe('generateId', () => {
  it('should generate unique IDs', () => {
    const id1 = generateId();
    const id2 = generateId();

    expect(id1).toBeTruthy();
    expect(id2).toBeTruthy();
    expect(id1).not.toBe(id2);
  });

  it('should generate string IDs', () => {
    const id = generateId();
    expect(typeof id).toBe('string');
    expect(id.length).toBeGreaterThan(0);
  });
});

describe('JSON utilities', () => {
  describe('safeJsonParse', () => {
    it('should parse valid JSON', () => {
      const result = safeJsonParse('{"key": "value"}', {});
      expect(result).toEqual({ key: 'value' });
    });

    it('should return fallback on invalid JSON', () => {
      const fallback = { default: true };
      const result = safeJsonParse('invalid json', fallback);
      expect(result).toBe(fallback);
    });
  });

  describe('safeJsonStringify', () => {
    it('should stringify valid objects', () => {
      const result = safeJsonStringify({ key: 'value' });
      expect(result).toBe('{"key":"value"}');
    });

    it('should return fallback on circular references', () => {
      const obj: any = {};
      obj.circular = obj;

      const result = safeJsonStringify(obj, '{"error":true}');
      expect(result).toBe('{"error":true}');
    });
  });
});

describe('isEmpty', () => {
  it('should return true for empty values', () => {
    expect(isEmpty(null)).toBe(true);
    expect(isEmpty(undefined)).toBe(true);
    expect(isEmpty('')).toBe(true);
    expect(isEmpty('   ')).toBe(true);
    expect(isEmpty([])).toBe(true);
    expect(isEmpty({})).toBe(true);
  });

  it('should return false for non-empty values', () => {
    expect(isEmpty('hello')).toBe(false);
    expect(isEmpty([1, 2])).toBe(false);
    expect(isEmpty({ key: 'value' })).toBe(false);
    expect(isEmpty(0)).toBe(false);
    expect(isEmpty(false)).toBe(false);
  });
});

describe('omit', () => {
  it('should omit specified keys', () => {
    const obj = { a: 1, b: 2, c: 3 };
    const result = omit(obj, ['b', 'c']);

    expect(result).toEqual({ a: 1 });
    expect(result).not.toHaveProperty('b');
    expect(result).not.toHaveProperty('c');
  });

  it('should not modify original object', () => {
    const obj = { a: 1, b: 2 };
    omit(obj, ['b']);

    expect(obj).toEqual({ a: 1, b: 2 });
  });
});

describe('pick', () => {
  it('should pick specified keys', () => {
    const obj = { a: 1, b: 2, c: 3 };
    const result = pick(obj, ['a', 'c']);

    expect(result).toEqual({ a: 1, c: 3 });
    expect(result).not.toHaveProperty('b');
  });

  it('should handle missing keys', () => {
    const obj = { a: 1, b: 2 };
    const result = pick(obj, ['a', 'c'] as any);

    expect(result).toEqual({ a: 1 });
  });
});
