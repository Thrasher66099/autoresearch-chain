# AutoResearch Chain — development task runner
#
# Usage:
#   make check      — run all checks (format, lint, test)
#   make build      — build the Rust workspace
#   make test       — run all tests
#   make fmt        — format Rust and Python code
#   make lint       — lint Rust and Python code
#   make sim        — run simulator (once implemented)
#   make clean      — remove build artifacts

.PHONY: check build test fmt lint sim clean \
        rust-build rust-test rust-fmt rust-lint \
        py-test py-fmt py-lint

# --------------------------------------------------------------------------
# Top-level targets
# --------------------------------------------------------------------------

check: fmt lint test

build: rust-build

test: rust-test py-test

fmt: rust-fmt py-fmt

lint: rust-lint py-lint

sim:
	@echo "Simulator not yet implemented."
	@echo "This target will run arc-simulator scenarios once Phase 0 is complete."
	@exit 1

clean:
	cargo clean
	find python -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find python -type d -name '*.egg-info' -exec rm -rf {} + 2>/dev/null || true

# --------------------------------------------------------------------------
# Rust targets
# --------------------------------------------------------------------------

rust-build:
	cargo build --workspace

rust-test:
	cargo test --workspace

rust-fmt:
	cargo fmt --all

rust-lint:
	cargo clippy --workspace -- -D warnings

# --------------------------------------------------------------------------
# Python targets
# --------------------------------------------------------------------------

py-test:
	cd python && python -m pytest

py-fmt:
	cd python && python -m ruff format .

py-lint:
	cd python && python -m ruff check .
