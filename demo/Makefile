.PHONY: help setup keygen deploy spawn build demo-keygen demo-share demo-recover demo-all

WALLET ?= wallet.json

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

setup: ## Install ao/ dependencies (yarn)
	cd ../ao && yarn install

keygen: ## Generate an Arweave wallet via ao/ scripts
	cd ../ao && yarn keygen $(WALLET)

deploy: ## Deploy WASM module to Arweave (returns module_id)
	cd ../ao && yarn deploy --wallet $(WALLET)

spawn: ## Spawn an AO process (returns process_id)
	cd ../ao && yarn instantiate --wallet $(WALLET)

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

demo-all: build ## Run full demo flow (keygen + share)
	@echo "=========================================="
	@echo " FORMIX Full Demo Flow"
	@echo "=========================================="
	cargo run --release -- keygen --role requester
	cargo run --release -- keygen --role owner
	@echo ""
	cargo run --release -- share
