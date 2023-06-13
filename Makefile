##
# This file is part of the IVMS Online.
#
# @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
##

SHELL:=bash

default: build

init: init-rust init-cargo

init-rust:
	sudo apt install libssl-dev musl-tools
	curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly --component rustfmt clippy

init-cargo:
	cargo install cargo-strip --version 0.2.3
	cargo install cargo-udeps --version 0.1.40
	cargo install cargo-tarpaulin --version 0.25.2
	cargo install cargo-workspaces --version 0.2.42

init-local:
	cargo install cargo-audit --version 0.17.6

clean:
	cargo clean

build:
	cargo build --release --target x86_64-unknown-linux-musl
	cargo strip

build-dev:
	cargo build --target x86_64-unknown-linux-musl

package: $(shell find target/x86_64-unknown-linux-musl/release/ -maxdepth 1 -executable -type f | sed s@x86_64-unknown-linux-musl/release/\\\(.*\\\)\${$}@\\1.zip@)

test:
	cargo tarpaulin --all-features --out Xml --lib --bins

test-local:
	docker run -d --rm --name dynamodb -p 8000:8000 amazon/dynamodb-local:1.20.0 -jar DynamoDBLocal.jar -inMemory
	make test
	docker stop dynamodb

test-integration:
	cargo test --test "*"

check:
	cargo fmt --check -- --config max_width=120,newline_style=Unix,edition=2021
	cargo clippy
	cargo udeps

check-local:
	cargo audit

doc:
	cargo doc --no-deps

# generic targets
target/%.zip: target/x86_64-unknown-linux-musl/release/%
	upx --best $<
	zip -j $@ $^
	printf "@ $(<F)\n@=bootstrap\n" | zipnote -w $@

full: clean build-dev test-local check check-local doc
