# CLI Test Suite

This directory contains comprehensive integration and unit tests for the LLM Memory Graph CLI.

## Structure

```
tests/
├── common/
│   └── mod.rs              # Shared test utilities
└── integration/
    ├── stats_test.rs       # Database statistics tests
    ├── session_test.rs     # Session management tests
    ├── query_test.rs       # Query command tests
    ├── export_test.rs      # Export command tests
    ├── import_test.rs      # Import command tests
    ├── template_test.rs    # Template command tests
    ├── agent_test.rs       # Agent command tests
    └── output_test.rs      # Output format tests
```

## Test Count: 87 Integration Tests + 11 Unit Tests = 98 Total

## Running Tests

### All Tests
```bash
cargo test
```

### Specific Test File
```bash
cargo test --test stats_test
```

### Specific Test
```bash
cargo test test_stats_json_format
```

### With Output
```bash
cargo test -- --nocapture
```

## Test Utilities

The `common` module provides:

### TestDb
Temporary database wrapper with automatic cleanup:
```rust
let db = TestDb::new().await;
let session_id = db.create_test_session(None).await;
```

### Assertions
- `assert_output_contains()` - Verify output contains expected strings
- `assert_valid_json()` - Validate JSON output
- `assert_valid_yaml()` - Validate YAML output

## Writing New Tests

1. Choose appropriate test file or create new one
2. Use `TestDb` for database operations
3. Test all output formats if applicable
4. Test error conditions
5. Use descriptive test names

Example:
```rust
#[tokio::test]
async fn test_my_feature() {
    let db = TestDb::new().await;
    // Test setup
    // Execute command
    // Verify output
}
```

## Coverage

See [TESTING.md](../TESTING.md) for detailed coverage information.

- Stats: 7 tests (90% coverage)
- Session: 13 tests (85% coverage)
- Query: 11 tests (85% coverage)
- Export: 9 tests (80% coverage)
- Import: 8 tests (80% coverage)
- Template: 12 tests (85% coverage)
- Agent: 16 tests (90% coverage)
- Output: 11 tests (95% coverage)
