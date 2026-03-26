.PHONY: all fmt lint test build clean check clippy

all: fmt lint test build

fmt:
	cargo fmt --all

lint:
	cargo fix --broken-code --allow-dirty
	cargo clippy --fix --allow-dirty
	cargo clippy --all-targets --all-features -- -D warnings

clippy:
	cargo clippy --all-targets --all-features -- -W clippy::pedantic -W clippy::restriction

test:
	cargo test --all-targets --all-features

build:
	cargo build --all-targets --all-features

check:
	cargo check --all-targets --all-features

clean:
	cargo clean
