.PHONY: help build start install fmt check

PREFIX ?= /usr
BINDIR ?= $(PREFIX)/bin

help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  help          Show this help message"
	@echo "  build         Build the project"
	@echo "  start         Run the built binary"
	@echo "  install       Install the existing binary (run 'make build' first)"
	@echo "  fmt           Format the code"
	@echo "  check         Format, lint and test the code"

build:
	cargo build --release

start: build
	./target/release/ashell

install:
	@test -x target/release/ashell || { \
		echo "error: target/release/ashell not found; run 'make build' first" >&2; \
		exit 1; \
	}
	install -Dm755 target/release/ashell $(DESTDIR)$(BINDIR)/ashell

fmt:
	cargo fmt --all

check: fmt
	cargo check --workspace --all-features
	cargo clippy --workspace --all-features -- -D warnings
	cargo test --workspace --all-features
