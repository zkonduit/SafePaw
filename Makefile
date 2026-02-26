.DEFAULT_GOAL := install

.PHONY: install test test-rust test-solidity

install:
	cargo install --path .

test: test-rust test-solidity

test-rust:
	cargo fmt
	cargo clippy
	cargo nextest run

test-solidity:
	forge test

build-dev:
	cargo build

build:
	cargo build --release