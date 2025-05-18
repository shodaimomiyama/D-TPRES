# ================ Rust =================
.PHONY: check fmt clippy lint test all

check:
	cargo check

fmt:
	cargo fmt --all

clippy:
	cargo clippy -- -A dead_code -A clippy::module_inception -A unused_variables -A unused_imports -A unused_mut -A unused_assignments -D warnings

lint: fmt clippy

test:
	cargo test

all: check lint test
