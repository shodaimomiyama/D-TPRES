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
