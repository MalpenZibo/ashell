.PHONY: help build start install fmt check

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
	@echo "  check         Format, check and lint the code"

build:
	cargo build --release

start: build
	./target/release/ashell

install: build
	install -Dm755 target/release/ashell $(DESTDIR)$(BINDIR)/ashell

fmt:
	cargo fmt

check: fmt
	cargo check
	cargo clippy -- -D warnings
