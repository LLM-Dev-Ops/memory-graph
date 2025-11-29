# Contributing to LLM Memory Graph

Thank you for your interest in contributing to LLM Memory Graph! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Documentation](#documentation)
- [Submitting Changes](#submitting-changes)
- [Code Style](#code-style)
- [Release Process](#release-process)

## Code of Conduct

This project follows a Code of Conduct that all contributors are expected to adhere to. Please read it before contributing.

### Our Pledge

We are committed to making participation in this project a harassment-free experience for everyone, regardless of age, body size, disability, ethnicity, gender identity and expression, level of experience, nationality, personal appearance, race, religion, or sexual identity and orientation.

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- Node.js 16 or higher
- Git
- Protocol Buffers compiler (protoc)

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:

```bash
git clone https://github.com/YOUR_USERNAME/llm-memory-graph.git
cd llm-memory-graph
```

3. Add upstream remote:

```bash
git remote add upstream https://github.com/globalbusinessadvisors/llm-memory-graph.git
```

## Development Setup

### Rust Development

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run with examples
cargo run --example simple_chatbot

# Build documentation
cargo doc --open
```

### TypeScript Client Development

```bash
cd clients/typescript

# Install dependencies
npm install

# Build
npm run build

# Run tests
npm test

# Lint and format
npm run lint
npm run format

# Generate documentation
npm run docs
```

### Running the Server

```bash
# Start development server
cargo run --bin server

# Or with CLI
cargo run --bin llm-memory-graph -- server start
```

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feature/add-xyz` - New features
- `fix/issue-123` - Bug fixes
- `docs/update-readme` - Documentation updates
- `refactor/cleanup-xyz` - Code refactoring
- `test/add-xyz-tests` - Test additions

### Commit Messages

Follow conventional commits:

```
type(scope): brief description

Detailed explanation if needed.

Fixes #123
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance

Examples:

```
feat(client): add retry logic with exponential backoff

Implements automatic retry mechanism for failed requests
with configurable backoff parameters.

Fixes #45
```

```
docs(api): update TypeScript examples

Add comprehensive examples for template usage
and streaming APIs.
```

## Testing

### Rust Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration_tests
```

### TypeScript Tests

```bash
cd clients/typescript

# Run tests
npm test

# Watch mode
npm run test:watch

# Coverage
npm run test:coverage
```

### Integration Tests

```bash
# Start server
cargo run --bin server &

# Run integration tests
cargo test --test grpc_integration_tests

# Stop server
pkill server
```

## Documentation

### Code Documentation

#### Rust

```rust
/// Brief description
///
/// Detailed explanation with examples
///
/// # Examples
///
/// ```
/// use llm_memory_graph::Client;
///
/// let client = Client::new(config).await?;
/// ```
///
/// # Errors
///
/// Returns error if connection fails
pub async fn connect(&self) -> Result<()> {
    // implementation
}
```

#### TypeScript

```typescript
/**
 * Brief description
 *
 * Detailed explanation with examples
 *
 * @param sessionId - The session identifier
 * @param content - The prompt content
 * @returns Promise resolving to the prompt node
 * @throws {ValidationError} If parameters are invalid
 *
 * @example
 * ```typescript
 * const prompt = await client.addPrompt({
 *   sessionId: session.id,
 *   content: 'Hello'
 * });
 * ```
 */
async addPrompt(request: AddPromptRequest): Promise<PromptNode> {
  // implementation
}
```

### Documentation Updates

When adding features:

1. Update API documentation in `docs/API.md`
2. Add examples in `docs/EXAMPLES.md`
3. Update relevant guides
4. Update CHANGELOG.md
5. Add JSDoc/Rustdoc comments

## Submitting Changes

### Pull Request Process

1. Update your branch with latest upstream:

```bash
git fetch upstream
git rebase upstream/main
```

2. Push to your fork:

```bash
git push origin your-branch-name
```

3. Create Pull Request on GitHub

4. Fill out the PR template

5. Wait for review

### PR Checklist

- [ ] Code follows project style guidelines
- [ ] Tests added/updated and passing
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Commit messages follow conventions
- [ ] No merge conflicts
- [ ] CI/CD checks passing

### Review Process

1. Automated checks run (tests, linting, formatting)
2. Code review by maintainers
3. Address feedback
4. Approval and merge

## Code Style

### Rust

Follow Rust standard style:

```bash
# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Check without building
cargo check
```

Guidelines:
- Use `rustfmt` defaults
- Follow Rust naming conventions
- Add comprehensive error handling
- Write idiomatic Rust
- Document public APIs

### TypeScript

Follow project ESLint/Prettier config:

```bash
cd clients/typescript

# Format code
npm run format

# Lint code
npm run lint

# Type check
npm run typecheck
```

Guidelines:
- Use TypeScript strict mode
- Prefer `const` over `let`
- Use meaningful variable names
- Add JSDoc comments
- Handle errors explicitly

### General Guidelines

- Keep functions small and focused
- Write self-documenting code
- Add comments for complex logic
- Use descriptive names
- Follow DRY principle
- Write testable code

## Project Structure

```
llm-memory-graph/
├── crates/
│   ├── llm-memory-graph/        # Core library
│   ├── llm-memory-graph-types/  # Shared types
│   ├── llm-memory-graph-client/ # Rust client
│   ├── llm-memory-graph-cli/    # CLI tool
│   └── llm-memory-graph-integrations/ # Integrations
├── clients/
│   └── typescript/              # TypeScript client
├── docs/                        # Documentation
├── examples/                    # Rust examples
├── proto/                       # Protocol buffers
├── tests/                       # Integration tests
└── scripts/                     # Build scripts
```

## Release Process

Releases are managed by maintainers:

1. Update version numbers
2. Update CHANGELOG.md
3. Create git tag
4. Build and test
5. Publish to package registries
6. Create GitHub release

## Getting Help

- GitHub Issues: Bug reports and feature requests
- Discussions: Questions and general discussion
- Documentation: Check existing docs first

## Recognition

Contributors will be:
- Listed in CHANGELOG.md
- Acknowledged in releases
- Added to contributors list

## License

By contributing, you agree that your contributions will be licensed under the project's MIT OR Apache-2.0 license.

## Questions?

Feel free to:
- Open an issue
- Start a discussion
- Reach out to maintainers

Thank you for contributing to LLM Memory Graph!
