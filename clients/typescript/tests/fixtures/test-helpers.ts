/**
 * Test helper functions and utilities
 */

/**
 * Mock gRPC client for testing
 */
export class MockGrpcClient {
  private mockMethods: Map<string, jest.Mock> = new Map();

  constructor() {
    this.setupMethods();
  }

  private setupMethods() {
    const methods = [
      'createSession',
      'getSession',
      'deleteSession',
      'listSessions',
      'createNode',
      'getNode',
      'updateNode',
      'deleteNode',
      'batchCreateNodes',
      'batchGetNodes',
      'createEdge',
      'getEdges',
      'deleteEdge',
      'query',
      'streamQuery',
      'addPrompt',
      'addResponse',
      'addToolInvocation',
      'createTemplate',
      'instantiateTemplate',
      'streamEvents',
      'subscribeToSession',
      'health',
      'getMetrics',
      'close',
    ];

    for (const method of methods) {
      this.mockMethods.set(method, jest.fn());
    }
  }

  getMock(method: string): jest.Mock {
    const mock = this.mockMethods.get(method);
    if (!mock) {
      throw new Error(`Mock method ${method} not found`);
    }
    return mock;
  }

  setupMethod(method: string, implementation: (...args: any[]) => any) {
    const mock = this.getMock(method);
    mock.mockImplementation(implementation);
  }

  setupMethodOnce(method: string, implementation: (...args: any[]) => any) {
    const mock = this.getMock(method);
    mock.mockImplementationOnce(implementation);
  }

  setupMethodResolve(method: string, value: any) {
    this.setupMethod(method, (req: any, callback: any) => {
      callback(null, value);
    });
  }

  setupMethodReject(method: string, error: any) {
    this.setupMethod(method, (req: any, callback: any) => {
      callback(error, null);
    });
  }

  reset() {
    for (const mock of this.mockMethods.values()) {
      mock.mockReset();
    }
  }

  clear() {
    for (const mock of this.mockMethods.values()) {
      mock.mockClear();
    }
  }
}

/**
 * Wait for a condition to be true
 */
export async function waitFor(
  condition: () => boolean,
  timeoutMs = 5000,
  intervalMs = 100
): Promise<void> {
  const startTime = Date.now();

  while (!condition()) {
    if (Date.now() - startTime > timeoutMs) {
      throw new Error(`Timeout waiting for condition after ${timeoutMs}ms`);
    }
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
}

/**
 * Wait for a specific duration
 */
export function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Create a deferred promise
 */
export interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (error: any) => void;
}

export function createDeferred<T>(): Deferred<T> {
  let resolve: (value: T) => void;
  let reject: (error: any) => void;

  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return {
    promise,
    resolve: resolve!,
    reject: reject!,
  };
}

/**
 * Capture console output
 */
export class ConsoleCapture {
  private originalLog: typeof console.log;
  private originalWarn: typeof console.warn;
  private originalError: typeof console.error;
  private logs: string[] = [];
  private warnings: string[] = [];
  private errors: string[] = [];

  constructor() {
    this.originalLog = console.log;
    this.originalWarn = console.warn;
    this.originalError = console.error;
  }

  start() {
    this.logs = [];
    this.warnings = [];
    this.errors = [];

    console.log = (...args: any[]) => {
      this.logs.push(args.join(' '));
    };

    console.warn = (...args: any[]) => {
      this.warnings.push(args.join(' '));
    };

    console.error = (...args: any[]) => {
      this.errors.push(args.join(' '));
    };
  }

  stop() {
    console.log = this.originalLog;
    console.warn = this.originalWarn;
    console.error = this.originalError;
  }

  getLogs() {
    return this.logs;
  }

  getWarnings() {
    return this.warnings;
  }

  getErrors() {
    return this.errors;
  }

  clear() {
    this.logs = [];
    this.warnings = [];
    this.errors = [];
  }
}

/**
 * Mock timer utilities
 */
export function advanceTimersByTime(ms: number) {
  jest.advanceTimersByTime(ms);
}

export function runAllTimers() {
  jest.runAllTimers();
}

export function clearAllTimers() {
  jest.clearAllTimers();
}

/**
 * Assert that a function throws with a specific error type and message
 */
export function assertThrows<T extends Error>(
  fn: () => any,
  errorType: new (...args: any[]) => T,
  messageContains?: string
): T {
  try {
    fn();
    throw new Error('Expected function to throw');
  } catch (error) {
    if (!(error instanceof errorType)) {
      throw new Error(
        `Expected error to be instance of ${errorType.name}, but got ${error.constructor.name}`
      );
    }
    if (messageContains && !error.message.includes(messageContains)) {
      throw new Error(
        `Expected error message to contain "${messageContains}", but got "${error.message}"`
      );
    }
    return error;
  }
}

/**
 * Assert that an async function throws with a specific error type and message
 */
export async function assertThrowsAsync<T extends Error>(
  fn: () => Promise<any>,
  errorType: new (...args: any[]) => T,
  messageContains?: string
): Promise<T> {
  try {
    await fn();
    throw new Error('Expected function to throw');
  } catch (error) {
    if (!(error instanceof errorType)) {
      throw new Error(
        `Expected error to be instance of ${errorType.name}, but got ${error.constructor.name}`
      );
    }
    if (messageContains && !error.message.includes(messageContains)) {
      throw new Error(
        `Expected error message to contain "${messageContains}", but got "${error.message}"`
      );
    }
    return error;
  }
}
