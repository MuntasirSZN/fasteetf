# justfile for changelogen-rs development

tools := "cargo-nextest cargo-deny cargo-llvm-cov cargo-hack"

# Commands

format := "cargo fmt --all"
clippy := "cargo clippy --all-targets --all-features"
coverage := "cargo llvm-cov --all-features --workspace"
build := "cargo build --all-features --locked"
nextest := "cargo hack nextest run --locked --optional-deps --each-feature --features compression --no-tests pass"

# Default recipe (shows help)
_default:
    @just --list

# Run miri
miri:
    rustup component add --toolchain=nightly miri
    rustup run nightly -- cargo miri nextest run --all-features

# Format code with rustfmt
fmt:
    {{ format }}

# Check code formatting
fmt-check:
    {{ format }} -- --check

# Run clippy linter
lint:
    {{ clippy }} -- -D warnings

# Run clippy linter (per-feature with cargo-hack)
lint-features:
    cargo hack clippy --locked --optional-deps --each-feature --features compression -- -D warnings

# Run clippy linter and autofix
lint-fix:
    {{ clippy }} --fix -- -D warnings

# Build the project
build:
    {{ build }}

# Build release version
build-release:
    {{ build }} --release

# Check compilation per-feature with cargo-hack
check-features:
    cargo hack check --locked --optional-deps --each-feature --features compression

# Run tests with nextest
test:
    {{ nextest }}
    cargo test --doc --locked

# Run tests without doc tests
test-fast:
    {{ nextest }}

# Run benchmarks locally
bench:
    cargo bench

# Generate documentation
doc:
    cargo doc --all --no-deps --open

# Generate documentation (CI variant, no open)
docs-ci:
    cargo doc --no-deps --no-default-features --locked
    cargo doc --no-deps --locked
    cargo doc --no-deps --all-features --locked

# Run all checks (format, lint, test)
check: fmt-check lint test

# Run pre-commit checks
pre-commit: fmt lint test deny
    @echo "{{ GREEN + BOLD }}✅ All pre-commit checks passed!{{ NORMAL }}"

# Run cargo-deny checks
deny:
    cargo deny check

# Install development tools using cargo-binstall
install-tools:
    @echo "{{ BLUE + BOLD }}Installing development tools...{{ NORMAL }}"
    cargo binstall -y {{ tools }} || cargo install cargo-binstall && cargo binstall -y {{ tools }}
    @echo "{{ GREEN + BOLD }}✅ Tools installed!{{ NORMAL }}"

# Install git pre-commit hook (run with just pre-commit)
install-hook:
    @echo -e "#!/bin/sh\njust pre-commit" > .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit
    @echo "{{ GREEN + BOLD }}✅ Pre-commit hook installed!{{ NORMAL }}"

# Clean build artifacts
clean:
    cargo clean

# Generate coverage report (text summary)
coverage:
    {{ coverage }}

# Generate coverage report (CI variant: nextest + doctest + lcov)
coverage-ci:
    cargo llvm-cov --all-features --no-report nextest
    cargo llvm-cov --all-features --no-report --doc
    cargo llvm-cov report --doctests --lcov --output-path lcov.info

# Generate HTML coverage report and open in browser
coverage-html:
    {{ coverage }} --html --open

# Generate lcov.info for Codecov (matches CI workflow)
coverage-lcov:
    {{ coverage }} --lcov --output-path lcov.info
    @echo "{{ GREEN + BOLD }}✅ Coverage report saved to lcov.info{{ NORMAL }}"

# Clean coverage data
coverage-clean:
    cargo llvm-cov clean

# Run all CI checks (format, lint per-feature, build, test, docs, audit)
ci-check: fmt-check lint-features build test docs-ci deny

# Create a release build and run the binary
release:
    cargo build --release
