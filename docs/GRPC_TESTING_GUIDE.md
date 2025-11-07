# gRPC Integration Testing Guide

Quick reference guide for running and maintaining the gRPC integration test suite.

## Quick Start

### 1. Setup Environment

```bash
# Run the automated setup script
./scripts/setup_grpc_tests.sh

# Or manually install protoc
# Ubuntu/Debian:
sudo apt-get install protobuf-compiler

# macOS:
brew install protobuf
```

### 2. Run Tests

```bash
# Run all gRPC integration tests
cargo test --test grpc_integration_test

# Run a specific test
cargo test --test grpc_integration_test test_health_check

# Run with output visible
cargo test --test grpc_integration_test -- --nocapture

# Run tests sequentially (helpful for debugging)
cargo test --test grpc_integration_test -- --test-threads=1
```

### 3. Check Results

Tests will output results for all 45 test cases covering:
- Health & Metrics (2 tests)
- Session Management (5 tests)
- Prompt & Response Operations (5 tests)
- Node Operations (6 tests)
- Edge Operations (5 tests)
- Error Handling (2 tests)
- Concurrent Operations (3 tests)
- Data Integrity (2 tests)
- Performance (2 tests)
- Plus 13 tests for unimplemented features

## Test Architecture

### TestServer Helper

Each test uses an isolated test server:

```rust
let server = TestServer::new().await.expect("Failed to start server");
let mut client = server.client.clone();

// Perform test operations...

server.shutdown().await;
```

**Features:**
- Random port allocation (no conflicts)
- In-memory database (test isolation)
- Automatic cleanup
- Built-in Prometheus metrics

### Test Data Helpers

```rust
// Session metadata
let metadata = test_metadata();

// Prompt metadata
let prompt_meta = test_prompt_metadata();

// Token usage
let usage = test_token_usage();

// Response metadata
let response_meta = test_response_metadata();
```

## Writing New Tests

### Basic Test Template

```rust
#[tokio::test]
async fn test_your_feature() {
    // 1. Setup
    let server = TestServer::new().await.expect("Failed to start server");
    let mut client = server.client.clone();

    // 2. Execute operation
    let request = Request::new(YourRequest {
        // ... request fields
    });
    let response = client.your_method(request).await.expect("Operation failed");

    // 3. Validate
    let result = response.into_inner();
    assert!(!result.id.is_empty());
    assert_eq!(result.field, expected_value);

    // 4. Cleanup
    server.shutdown().await;
}
```

### Testing Error Cases

```rust
#[tokio::test]
async fn test_error_handling() {
    let server = TestServer::new().await.expect("Failed to start server");
    let mut client = server.client.clone();

    let request = Request::new(InvalidRequest { /* ... */ });
    let response = client.method(request).await;

    assert!(response.is_err());
    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);

    server.shutdown().await;
}
```

### Testing Concurrent Operations

```rust
#[tokio::test]
async fn test_concurrent_operations() {
    let server = TestServer::new().await.expect("Failed to start server");

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let mut client = server.client.clone();
            tokio::spawn(async move {
                // Perform operation
                client.method(request).await
            })
        })
        .collect();

    for handle in handles {
        let result = handle.await.expect("Task failed");
        assert!(result.is_ok());
    }

    server.shutdown().await;
}
```

## Common Test Patterns

### 1. Full Workflow Test

Tests complete user journey:

```rust
// Create session → Add prompt → Add response → Verify
let session = create_session(&mut client).await;
let prompt = add_prompt(&mut client, &session).await;
let response = add_response(&mut client, &prompt).await;
verify_relationships(&session, &prompt, &response);
```

### 2. Data Integrity Test

Verifies relationships and consistency:

```rust
assert_eq!(response.prompt_id, prompt.id);
assert_eq!(prompt.session_id, session.id);
```

### 3. Performance Test

Measures operation timing:

```rust
let start = std::time::Instant::now();
perform_operation().await;
let duration = start.elapsed();
assert!(duration < Duration::from_millis(100));
```

### 4. Batch Operation Test

Tests bulk operations:

```rust
let ids = create_multiple_items(&mut client, 50).await;
let batch_result = batch_get(&mut client, ids).await;
assert_eq!(batch_result.len(), 50);
```

## Debugging Tests

### Enable Detailed Logging

```bash
# Set log level
RUST_LOG=debug cargo test --test grpc_integration_test

# Or for specific module
RUST_LOG=llm_memory_graph::grpc=trace cargo test --test grpc_integration_test
```

### Run Single Test with Output

```bash
cargo test --test grpc_integration_test test_name -- --nocapture --test-threads=1
```

### Debug Server Issues

Add debugging in test:

```rust
#[tokio::test]
async fn test_debug() {
    let server = TestServer::new().await.expect("Failed to start server");
    println!("Server address: {}", server.address);

    // Add breakpoint or additional logging here
    dbg!(&server.client);

    server.shutdown().await;
}
```

## Test Maintenance

### Adding Tests for New Features

1. **Identify the feature category** (Session, Node, Edge, etc.)
2. **Write positive test case** (happy path)
3. **Write negative test cases** (error scenarios)
4. **Add concurrent test** if applicable
5. **Update test count** in documentation

### Updating Existing Tests

When implementation changes:

1. **Review affected tests** - grep for the changed method
2. **Update assertions** - match new behavior
3. **Add new validations** - for new fields/features
4. **Verify backwards compatibility** - ensure existing tests pass

### Handling Flaky Tests

If tests fail intermittently:

1. **Add delays** for timing-sensitive operations:
   ```rust
   tokio::time::sleep(Duration::from_millis(100)).await;
   ```

2. **Increase timeouts** if needed:
   ```rust
   tokio::time::timeout(Duration::from_secs(5), operation).await
   ```

3. **Check for race conditions** in concurrent tests

4. **Isolate test** to run alone:
   ```bash
   cargo test --test grpc_integration_test problematic_test -- --test-threads=1
   ```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: gRPC Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install protoc
        run: sudo apt-get install -y protobuf-compiler

      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Run gRPC tests
        run: cargo test --test grpc_integration_test

      - name: Generate test report
        if: always()
        run: |
          cargo test --test grpc_integration_test -- --format json > test-results.json
```

### Local Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

echo "Running gRPC integration tests..."
cargo test --test grpc_integration_test --quiet

if [ $? -ne 0 ]; then
    echo "Tests failed! Commit aborted."
    exit 1
fi
```

## Performance Benchmarking

### Run Performance Tests Only

```bash
# Filter to performance tests
cargo test --test grpc_integration_test performance
```

### Measure with Criterion (Future)

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_grpc_operations(c: &mut Criterion) {
    c.bench_function("create_session", |b| {
        b.iter(|| {
            // Benchmark code
        })
    });
}
```

## Troubleshooting

### Problem: protoc not found

**Solution:**
```bash
# Install protoc
sudo apt-get install protobuf-compiler  # Ubuntu
brew install protobuf                    # macOS
```

### Problem: Port already in use

**Solution:**
Tests use random ports, but if you see this error:
- Kill existing processes on gRPC ports
- Restart the test
- Check for zombie test processes

### Problem: Tests timeout

**Solution:**
- Increase timeout in test configuration
- Check system resources
- Run with `--test-threads=1` to reduce load

### Problem: Build fails with proto errors

**Solution:**
```bash
# Clean and rebuild
cargo clean
cargo build

# Verify proto file
cat proto/memory_graph.proto | head -20
```

### Problem: Tests fail after code changes

**Solution:**
1. Review change impact
2. Update test expectations
3. Verify protobuf definitions match
4. Check type conversions

## Best Practices

### ✅ DO

- Use `TestServer::new()` for test isolation
- Clean up resources with `server.shutdown()`
- Test both success and failure cases
- Use descriptive test names
- Add comments for complex test logic
- Validate all important fields
- Test concurrent scenarios
- Measure performance

### ❌ DON'T

- Share state between tests
- Use fixed ports (use random ports)
- Skip cleanup (causes resource leaks)
- Test only happy paths
- Make tests depend on execution order
- Use hardcoded IDs
- Ignore timing issues
- Skip error validation

## Resources

- **Test File:** `tests/grpc_integration_test.rs`
- **Test Report:** `docs/GRPC_INTEGRATION_TEST_REPORT.md`
- **Proto Definition:** `proto/memory_graph.proto`
- **Service Implementation:** `src/grpc/service.rs`
- **Setup Script:** `scripts/setup_grpc_tests.sh`

## Support

For questions or issues:

1. Check test output for error details
2. Review test report documentation
3. Check proto definitions
4. Verify service implementation
5. Enable debug logging

## Test Statistics

- **Total Tests:** 45
- **Test Categories:** 13
- **Coverage:** 100% of implemented features
- **Concurrent Test Load:** Up to 20 concurrent operations
- **Performance SLAs:** Health < 100ms, Operations < 500ms

---

**Last Updated:** 2025-11-07
**Test Suite Version:** 1.0.0
**Minimum Rust Version:** 1.70+
