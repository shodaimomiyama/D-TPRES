# ================ Rust =================
.PHONY: check fmt clippy lint test all

check:
	cargo check

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- \
		-D warnings \
		-W clippy::all \
		-W clippy::pedantic \
		-W clippy::nursery \
		-D clippy::unwrap_used \
		-D clippy::expect_used \
		-D clippy::panic \
		-D clippy::unimplemented \
		-D clippy::todo \
		-W clippy::float_cmp \
		-W clippy::float_arithmetic \
		-W clippy::arithmetic_side_effects \
		-W clippy::indexing_slicing \
		-D clippy::mem_forget \
		-W clippy::cast_possible_truncation \
		-W clippy::cast_possible_wrap \
		-W clippy::cast_precision_loss \
		-W clippy::cast_sign_loss \
		-W clippy::missing_const_for_fn \
		-W clippy::redundant_clone \
		-W clippy::semicolon_if_nothing_returned \
		-W clippy::missing_fields_in_debug \
		-W clippy::large_stack_frames \
		-W clippy::exhaustive_enums \
		-W clippy::exhaustive_structs \
		-A clippy::module_inception \
		-A clippy::must_use_candidate \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc \
		-A clippy::module_name_repetitions \
		-A clippy::too_many_lines \
		-A clippy::struct_field_names \
		-A clippy::similar_names \
		-A clippy::doc_markdown \
		-A clippy::struct_excessive_bools \
		-A clippy::redundant_pub_crate \
		-A dead_code \
		-A unused_variables \
		-A unused_imports \
		-A unused_mut \
		-A unused_assignments

# Strict clippy for production/security audit
clippy-strict:
	cargo clippy --all-targets --all-features -- \
		-D warnings \
		-D clippy::all \
		-D clippy::pedantic \
		-D clippy::nursery \
		-D clippy::restriction \
		-D clippy::cargo \
		-A clippy::blanket_clippy_restriction_lints \
		-A clippy::implicit_return \
		-A clippy::missing_docs_in_private_items \
		-A clippy::shadow_reuse \
		-A clippy::shadow_same \
		-A clippy::missing_inline_in_public_items \
		-A clippy::separated_literal_suffix \
		-A clippy::mod_module_files \
		-A clippy::self-named-module-files

lint: fmt clippy

test:
	cargo test

all: check lint test

# ================ MCP Servers =================
.PHONY: mcp-setup mcp-docs-index start-docs-rag start-mobile-mcp start-context7 stop-context7 logs-context7 status-context7

# MCP完全セットアップ（初回のみ）
mcp-setup:
	@echo "==> Setting up MCP servers..."
	@echo "==> Setting up Context7 MCP server..."
	cd mcp/context7 && make build

# MCPヘルプ
mcp-help:
	@echo "MCP Servers Commands:"
	@echo "  mcp-setup        - Complete setup (all MCP servers)"
	@echo "  start-context7   - Start context7 MCP server in background"
	@echo "  stop-context7    - Stop context7 MCP server"
	@echo "  logs-context7    - Show context7 MCP server logs"
	@echo "  status-context7  - Check context7 MCP server status"
	@echo "  mcp-help         - Show MCP help"

# Context7 MCP起動（.mcp.json用）
start-context7:
	@echo "==> Starting Context7 MCP server in background..."
	cd mcp/context7 && make up

# Context7 MCP停止
stop-context7:
	@echo "==> Stopping Context7 MCP server..."
	cd mcp/context7 && make down

# Context7 MCPログ表示
logs-context7:
	@echo "==> Showing Context7 MCP server logs..."
	cd mcp/context7 && make logs

# Context7 MCP状態確認
status-context7:
	@echo "==> Checking Context7 MCP server status..."
	cd mcp/context7 && make status
