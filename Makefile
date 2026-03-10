.DEFAULT_GOAL := default

.PHONY: default install start test test-rust test-solidity build-dev build

default: install start

install:
	cargo install --path .

start:
	safepaw start

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
