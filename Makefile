.PHONY: all fmt lint test build clean check clippy install

all: fmt lint test build

fmt:
	cargo fmt --all

lint:
	cargo fix --broken-code --allow-dirty
	cargo clippy --fix --allow-dirty
	cargo clippy --all-targets --all-features -- -D warnings

clippy:
	cargo clippy --all-targets --all-features

test:
	cargo test --all-targets --all-features --quiet

build:
	cargo build --all-targets --all-features

check:
	cargo check --all-targets --all-features

clean:
	cargo clean

install:
	cargo install --path .

release:
	cargo release
