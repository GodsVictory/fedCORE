# Makefile for FedCore Platform CLI

OCI_REGISTRY ?= registry.example.com

# Default target
.PHONY: help
help:
	@echo "FedCore Platform CLI - Makefile targets:"
	@echo ""
	@echo "  make build          - Build the CLI in debug mode"
	@echo "  make release        - Build the CLI in release mode (optimized)"
	@echo "  make build-all      - Build release binaries for Linux and Windows"
	@echo "  make download-tools - Download dependency CLIs for Linux and Windows"
	@echo "  make stage          - Build + bundle CLI and tools, push to registry"
	@echo "  make install        - Install the CLI to /usr/local/bin"
	@echo "  make test           - Run tests"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make run-validate   - Run the validate command"
	@echo "  make run-matrix     - Run the matrix command"
	@echo "  make fmt            - Format code"
	@echo "  make clippy         - Run linter"
	@echo ""

.PHONY: build
build:
	cargo build

.PHONY: release
release:
	cargo build --release

.PHONY: stage
stage: build-all
	cp target/release/fedcore tools/linux/fedcore
	cp target/x86_64-pc-windows-gnu/release/fedcore.exe tools/windows/fedcore.exe
	cd tools/linux && oras push $(OCI_REGISTRY)/fedcore/cli-linux:latest \
		fedcore crane flux helm kbld kubectl ytt
	cd tools/windows && oras push $(OCI_REGISTRY)/fedcore/cli-windows:latest \
		fedcore.exe crane.exe flux.exe helm.exe kbld.exe kubectl.exe ytt.exe
	rm tools/linux/fedcore tools/windows/fedcore.exe

.PHONY: build-all
build-all:
	cargo build --release
	cargo build --release --target x86_64-pc-windows-gnu

.PHONY: install
install: release
	@echo "Installing fedcore to /usr/local/bin/fedcore..."
	sudo cp target/release/fedcore /usr/local/bin/fedcore
	@echo "✓ Installation complete. Run 'fedcore --help' to get started."

.PHONY: test
test:
	cargo test

.PHONY: clean
clean:
	cargo clean

.PHONY: run-validate
run-validate: build
	./target/debug/fedcore validate

.PHONY: run-matrix
run-matrix: build
	./target/debug/fedcore matrix

.PHONY: fmt
fmt:
	cargo fmt

.PHONY: clippy
clippy:
	cargo clippy -- -D warnings

# Script compatibility targets (using the CLI instead of bash)
.PHONY: bootstrap
bootstrap: build
	@echo "Usage: make bootstrap CLUSTER=platform/clusters/your-cluster [DEPLOY=true]"
	@if [ -z "$(CLUSTER)" ]; then \
		echo "Error: CLUSTER variable is required"; \
		echo "Example: make bootstrap CLUSTER=platform/clusters/my-cluster"; \
		exit 1; \
	fi
	@if [ "$(DEPLOY)" = "true" ]; then \
		./target/debug/fedcore bootstrap -c $(CLUSTER) --deploy; \
	else \
		./target/debug/fedcore bootstrap -c $(CLUSTER); \
	fi

.PHONY: validate
validate: run-validate

.PHONY: matrix
matrix: run-matrix
