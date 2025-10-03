.PHONY: help build start install fmt check

help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  help          Show this help message"
	@echo "  build         Build the project
	@echo "  start         Run the build"
	@echo "  install       Install the build to /usr/bin"
	@echo "  fmt           Format the code"
	@echo "  check         Format, check and lint the code"

build:
	cargo build --release

start: build-release
	./target/release/ashell

install: build
	sudo cp -f target/release/ashell /usr/bin

fmt:
	cargo fmt

check: fmt
	cargo check
	cargo clippy -- -D warnings