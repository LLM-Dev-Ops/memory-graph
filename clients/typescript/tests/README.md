# TypeScript Client Test Suite

This directory contains comprehensive tests for the TypeScript Memory Graph client.

## Test Structure

```
tests/
├── unit/                  # Unit tests for individual modules
│   ├── client.test.ts     # Client tests
│   ├── errors.test.ts     # Error handling tests
│   ├── retry.test.ts      # Retry logic tests
│   ├── validators.test.ts # Input validation tests
│   └── utils.test.ts      # Utility function tests
├── integration/           # Integration tests requiring server
│   └── client-integration.test.ts
├── fixtures/              # Test data and helpers
│   ├── mock-data.ts       # Mock data generators
│   └── test-helpers.ts    # Test utilities
└── README.md
```

## Running Tests

### Run All Tests
```bash
npm test
```

### Run Tests with Coverage
```bash
npm run test:coverage
```

### Run Unit Tests Only
```bash
npm run test:unit
```

### Run Integration Tests
Integration tests require a running Memory Graph server:

```bash
# Start the Memory Graph server first, then:
INTEGRATION_TESTS=true npm run test:integration
```

### Watch Mode
```bash
npm run test:watch
```

## Test Coverage Goals

The test suite aims for 80%+ coverage across all metrics:
- **Branches**: 80%+
- **Functions**: 80%+
- **Lines**: 80%+
- **Statements**: 80%+

## Test Categories

### Unit Tests

#### errors.test.ts (90+ test cases)
- All error class constructors
- Error property validation
- gRPC error mapping for all status codes
- Type guards (isMemoryGraphError, isRetryableError)
- Error serialization

#### retry.test.ts (50+ test cases)
- Exponential backoff calculation
- Retry with backoff logic
- Custom retry policies
- RetryManager with context
- CircuitBreaker states and transitions
- Jitter in backoff delays

#### validators.test.ts (100+ test cases)
- String validation (non-empty, UUID format)
- Number validation (positive, range, integers)
- Date validation
- Object validation
- Enum validation
- Metadata validation
- Token usage validation
- Prompt/Response metadata validation
- Request validation (AddPrompt, AddResponse, Query)
- Node and Edge validation
- Array validation
- Input sanitization
- JSON validation

#### utils.test.ts (80+ test cases)
- Time utilities (sleep, debounce, throttle)
- Timestamp formatting and parsing
- Duration formatting
- Token usage calculations
- Type guards for node types
- Filtering and sorting operations
- Array chunking and batch processing
- Deep clone and merge
- Query string conversion
- ID generation
- JSON utilities
- isEmpty checks
- Object manipulation (omit, pick)

#### client.test.ts (40+ test cases)
- Client construction with various configs
- Connection management
- Retry policy configuration
- Health checks
- Configuration getters
- Error handling
- TLS configuration
- Address handling
- Input validation for all methods

### Integration Tests

#### client-integration.test.ts
- Full end-to-end workflows
- Session lifecycle (create, get, list, delete)
- Prompt and response operations
- Query operations with filters
- Error handling with real server
- Connection management

## Mock Data and Helpers

### mock-data.ts
Provides pre-configured mock objects:
- `mockSession`: Sample session data
- `mockPromptNodeData`: Sample prompt node
- `mockResponseNodeData`: Sample response node
- `mockTokenUsage`: Sample token usage
- Generator functions for creating custom mocks
- gRPC error generator

### test-helpers.ts
Provides testing utilities:
- `MockGrpcClient`: Mock gRPC client for testing
- `waitFor`: Wait for conditions
- `createDeferred`: Create deferred promises
- `ConsoleCapture`: Capture console output
- `assertThrows`: Assert function throws specific error
- Timer manipulation helpers

## Writing New Tests

### Unit Test Template

```typescript
import { describe, it, expect } from '@jest/globals';
import { YourModule } from '../../src/your-module';

describe('YourModule', () => {
  describe('yourFunction', () => {
    it('should handle valid input', () => {
      const result = yourFunction('valid input');
      expect(result).toBe('expected output');
    });

    it('should throw on invalid input', () => {
      expect(() => yourFunction('')).toThrow(ValidationError);
    });
  });
});
```

### Integration Test Template

```typescript
import { describe, it, expect } from '@jest/globals';

const describeIntegration = process.env.INTEGRATION_TESTS === 'true'
  ? describe
  : describe.skip;

describeIntegration('Feature Integration', () => {
  it('should work end-to-end', async () => {
    // Test implementation
  });
});
```

## Best Practices

1. **Test Isolation**: Each test should be independent
2. **Clear Names**: Use descriptive test names
3. **Arrange-Act-Assert**: Structure tests clearly
4. **Mock External Dependencies**: Don't rely on external services in unit tests
5. **Test Error Cases**: Test both success and failure paths
6. **Use Type Guards**: Leverage TypeScript's type system
7. **Clean Up**: Always clean up resources (close clients, clear timers)

## Continuous Integration

Tests are automatically run on:
- Pull requests
- Pushes to main branch
- Nightly builds

Coverage reports are generated and uploaded for tracking.

## Troubleshooting

### Tests Timeout
Increase the timeout in jest.config.js:
```javascript
testTimeout: 20000
```

### Integration Tests Fail
Ensure the Memory Graph server is running:
```bash
# Check server status
curl http://localhost:50051/health

# Start server if needed
cd ../../server && cargo run
```

### Mock Issues
If mocks aren't working, check:
1. jest.mock() is called before imports
2. Mock implementation matches actual API
3. Mock is reset between tests

### Coverage Below Threshold
To see uncovered lines:
```bash
npm run test:coverage
# Open coverage/lcov-report/index.html in browser
```

## Resources

- [Jest Documentation](https://jestjs.io/)
- [Testing Best Practices](https://testingjavascript.com/)
- [TypeScript Testing Guide](https://www.typescriptlang.org/docs/handbook/testing.html)
