.PHONY: help setup keygen deploy spawn build demo-keygen demo-share demo-recover demo-all demo-local demo-local-share demo-local-reencrypt demo-local-recover demo-local-all

WALLET ?= ./arweave-keyfile.json

help: ## Show this help
	@echo "FORMIX Demo Targets:"
	@echo ""
	@echo "  Setup & Infrastructure:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "  Quick Start:"
	@echo "    1. cp .env.example .env && edit ARWEAVE_WALLET_PATH"
	@echo "    2. make setup"
	@echo "    3. make keygen"
	@echo "    4. make deploy"
	@echo "    5. make spawn"
	@echo "    6. cp deploy.example.json deploy.json && fill in IDs"
	@echo "    7. make demo-all"

setup: ## Install ao/ dependencies (npm)
	cd ../ao && npm install --ignore-scripts

keygen: ## Generate an Arweave wallet via ao/ scripts
	cd ../ao && npm run keygen -- $(WALLET)

deploy: ## Build WASM and deploy module to Arweave (returns module_id)
	cargo build --release --manifest-path ../ao/contracts/Cargo.toml --target wasm32-unknown-unknown
	cd ../ao && npm run deploy -- --wallet $(abspath $(WALLET))

spawn: ## Spawn an AO process (MODULE_ID, SCHEDULER required)
	@test -n "$(MODULE_ID)" || (echo "Error: MODULE_ID is required (from make deploy output)" && exit 1)
	@test -n "$(SCHEDULER)" || (echo "Error: SCHEDULER is required" && exit 1)
	cd ../ao && npm run instantiate -- --wallet $(abspath $(WALLET)) --module_id $(MODULE_ID) --scheduler $(SCHEDULER)

build: ## Build the demo binary (release)
	cargo build --release

demo-keygen: build ## Run keygen subcommand (ROLE=owner|requester required)
	@test -n "$(ROLE)" || (echo "Error: ROLE is required (owner|requester)" && exit 1)
	cargo run --release -- keygen --role $(ROLE)

demo-share: build ## Run share subcommand (Phase 1)
	cargo run --release -- share

demo-recover: build ## Run recover subcommand (Phase 3)
	@echo "Usage: make demo-recover SECRET_ID=<id>"
	@test -n "$(SECRET_ID)" || (echo "Error: SECRET_ID is required" && exit 1)
	cargo run --release -- recover --secret-id $(SECRET_ID)

demo-local-share: build ## Local Phase 1: split secret (REQUESTER_PUBKEY required)
	@test -n "$(REQUESTER_PUBKEY)" || (echo "Error: REQUESTER_PUBKEY required. Example: make demo-local-share REQUESTER_PUBKEY=\$$(jq -r .public_key_hex .formix-demo/requester.json)" && exit 1)
	cargo run --release -- local share --requester-pubkey $(REQUESTER_PUBKEY) $(if $(PLAINTEXT),--plaintext "$(PLAINTEXT)",)

demo-local-reencrypt: build ## Local Phase 2: re-encrypt via contract (SECRET_ID required)
	@test -n "$(SECRET_ID)" || (echo "Error: SECRET_ID required" && exit 1)
	cargo run --release -- local reencrypt --secret-id $(SECRET_ID)

demo-local-recover: build ## Local Phase 3: recover secret (SECRET_ID required)
	@test -n "$(SECRET_ID)" || (echo "Error: SECRET_ID required" && exit 1)
	cargo run --release -- local recover --secret-id $(SECRET_ID)

demo-local-all: build ## Run all local steps (keygen + share + reencrypt + recover)
	cargo run --release -- local all $(if $(PLAINTEXT),--plaintext "$(PLAINTEXT)",)

demo-local: demo-local-all ## Alias for demo-local-all (backward compat)

demo-all: build ## Run full demo flow (keygen + share)
	@echo "=========================================="
	@echo " FORMIX Full Demo Flow"
	@echo "=========================================="
	cargo run --release -- keygen --role requester
	cargo run --release -- keygen --role owner
	@echo ""
	cargo run --release -- share
