import { afterEach, describe, expect, it } from '@jest/globals';
import { deepMerge } from '../../src/utils';

afterEach(() => {
  delete (Object.prototype as Record<string, unknown>).mergeProbe;
});

describe('deepMerge destination ownership', () => {
  it('keeps __proto__ as inert own data without changing object prototypes', () => {
    const target: Record<string, any> = {};
    deepMerge(target, JSON.parse('{"__proto__":{"mergeProbe":true}}'));
    expect((Object.prototype as Record<string, unknown>).mergeProbe).toBeUndefined();
    expect(Object.getPrototypeOf(target)).toBe(Object.prototype);
    expect(Object.prototype.hasOwnProperty.call(target, '__proto__')).toBe(true);
    expect(target.__proto__.mergeProbe).toBe(true);
  });

  it('does not recurse through inherited constructor.prototype', () => {
    const target: Record<string, any> = {};
    deepMerge(target, JSON.parse('{"constructor":{"prototype":{"mergeProbe":true}}}'));
    expect((Object.prototype as Record<string, unknown>).mergeProbe).toBeUndefined();
    expect(target.constructor.prototype.mergeProbe).toBe(true);
  });

  it('does not mutate inherited nested destination objects', () => {
    const prototype = { options: { existing: 1 } };
    const target = Object.create(prototype);
    deepMerge(target, { options: { added: 2 } });
    expect(prototype.options).toEqual({ existing: 1 });
    expect(Object.prototype.hasOwnProperty.call(target, 'options')).toBe(true);
    expect(target.options).toEqual({ added: 2 });
  });

  it('protects nested destinations too', () => {
    const target = { config: {} };
    deepMerge(target, JSON.parse('{"config":{"__proto__":{"mergeProbe":true}}}'));
    expect((Object.prototype as Record<string, unknown>).mergeProbe).toBeUndefined();
    expect(Object.getPrototypeOf(target.config)).toBe(Object.prototype);
  });
});
