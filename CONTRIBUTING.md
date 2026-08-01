# Contributing to fasteetf

Thank you for your interest in contributing to `fasteetf`! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites

- Rust nightly (for Miri tests)
- Rust stable (for regular development)
- `cargo-nextest` (recommended for faster tests)
- `just` (command runner)

### Installing Tools

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain
rustup toolchain install nightly

# Install Miri
rustup component add --toolchain nightly miri

# Install cargo-nextest
cargo install cargo-nextest

# Install just
cargo install just
```

### Building

```bash
# Build the project
just build

# Build with all features
cargo build --all-features
```

## Testing

### Running Tests

```bash
# Run all tests with nextest
just test

# Run tests with a specific feature set
cargo test --features "compression,serde"

# Run Miri tests (detects undefined behavior)
just miri
```

### Test Coverage

```bash
# Generate test coverage report
just coverage

# Generate HTML coverage report
just coverage-html
```

### Important Testing Notes

- **Miri tests**: Some tests that call foreign C functions (zlib) are skipped under Miri using `#[cfg(not(miri))]`. This is expected behavior.
- **Proptest tests**: Property-based tests are also skipped under Miri because proptest internally calls `getcwd` which Miri doesn't allow in isolated mode.
- **Compression tests**: Require zlib backend features to be enabled.

## Code Style

### Formatting

```bash
# Format code
just fmt

# Check formatting
just fmt-check
```

### Linting

```bash
# Run clippy
just lint

# Run clippy per-feature
just lint-features

# Run clippy with autofix
just lint-fix
```

### Guidelines

1. **Follow Rust conventions**: Use `rustfmt` and adhere to standard Rust naming conventions.
2. **Document public API**: All public functions, types, and traits must have doc comments.
3. **Keep it `no_std` compatible**: Core functionality should work without `std` or `alloc` unless the feature explicitly requires it.
4. **Minimize allocations**: Prefer zero-copy parsing and stack allocation where possible.
5. **Handle errors gracefully**: Use `EtfError` for all parse/encode errors.
6. **Test thoroughly**: Add tests for new functionality and ensure existing tests pass.

## Pull Request Process

1. **Fork the repository** and create a feature branch from `main`.
2. **Make your changes** following the code style guidelines.
3. **Add tests** for new functionality.
4. **Update documentation** if needed (README, doc comments, etc.).
5. **Run the test suite**:
   ```bash
   just check  # Runs fmt-check, lint, test
   ```
6. **Run Miri** to check for undefined behavior:
   ```bash
   just miri
   ```
7. **Commit your changes** with clear, descriptive commit messages.
8. **Push to your fork** and submit a pull request to the `main` branch.
9. **Describe your changes** in the PR description:
   - What problem does this solve?
   - What changes were made?
   - Are there any breaking changes?
   - Reference any related issues.

### PR Checklist

- [ ] Code is formatted with `rustfmt`
- [ ] No new clippy warnings (or existing ones fixed)
- [ ] Tests pass (`just test`)
- [ ] Miri tests pass (`just miri`)
- [ ] Documentation updated (if needed)
- [ ] CHANGELOG.md updated (for notable changes)
- [ ] All CI checks pass

## Feature Development

### Adding a New Feature

1. **Discuss first**: For significant changes, open an issue to discuss the design before implementing.
2. **Add a feature flag**: If the feature is optional, add it to `Cargo.toml` under `[features]`.
3. **Update documentation**: Add the feature to the feature matrix in `README.md` and `Cargo.toml` comments.
4. **Test thoroughly**: Add unit tests, integration tests, and consider property-based tests with proptest.

## Architecture Overview

### Core Components

- **`src/parser.rs`**: Zero-copy ETF parser
- **`src/encoder.rs`**: ETF encoder
- **`src/arena.rs`**: Bump allocator for zero-copy parsing
- **`src/types.rs`**: Term types (`Term`, `OwnedTerm`, etc.)
- **`src/zlib.rs`**: Zlib compression/decompression with trait-based backend
- **`src/error.rs`**: Error types (`EtfError`, `Needed`)
- **`src/lib.rs`**: Public API and re-exports

### Key Design Principles

1. **Zero-copy parsing**: Parse ETF into a tree of borrowed references using an arena allocator
2. **`no_std` first**: Core functionality works without `std` or `alloc`
3. **Feature modularity**: Optional features (compression, serde) are independently selectable
4. **Trait-based backends**: Zlib backend is pluggable via the `ZlibBackend` trait
5. **Safety**: No undefined behavior (verified with Miri)

## Reporting Issues

When reporting issues, please include:

1. **Description**: What went wrong?
2. **Reproduction steps**: Minimal code to reproduce the issue
3. **Expected behavior**: What should happen?
4. **Actual behavior**: What actually happened?
5. **Environment**: Rust version, OS, features enabled
6. **Additional context**: Related issues, possible solutions, etc.

## Getting Help

- **Open an issue**: For bugs, feature requests, or questions
- **Check existing issues**: Your question might already be answered
- **Review documentation**: See docs.rs/fasteetf for API docs

## License

By contributing to `fasteetf`, you agree that your contributions will be licensed under the LGPL-3.0-or-later license, the same license as the project.

## Recognition

Contributors will be acknowledged in release notes and the project README. Thank you for helping make `fasteetf` better!
