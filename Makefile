.PHONY: help build start install fmt check fmt-check

PREFIX ?= /usr
BINDIR ?= $(PREFIX)/bin

help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  help          Show this help message"
	@echo "  build         Build the project"
	@echo "  start         Run the build"
	@echo "  install       Install the build (supports DESTDIR and PREFIX)"
	@echo "  fmt           Format the code"
	@echo "  fmt-check     Check formatting without modifying files"
	@echo "  check         Check formatting, build, test and lint the code"

build:
	cargo build --release

start: build
	./target/release/ashell

install: build
	install -Dm755 target/release/ashell $(DESTDIR)$(BINDIR)/ashell

fmt:
	cargo fmt

check: fmt-check
	cargo check
	cargo test
	cargo clippy --all-targets -- -D warnings

fmt-check:
	cargo fmt -- --check
