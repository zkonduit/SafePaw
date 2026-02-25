.DEFAULT_GOAL := test

.PHONY: test test-rust test-solidity

test: test-rust test-solidity

test-rust:
	cargo nextest run

test-solidity:
	forge test

build-dev:
	cargo build

build:
	cargo build --release