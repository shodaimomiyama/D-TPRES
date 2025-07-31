.PHONY: help build up down logs shell

help:
	@echo "Available commands:"
	@echo "  make build  - Build the Serena Docker image"
	@echo "  make up     - Start the Serena container"
	@echo "  make down   - Stop the Serena container"
	@echo "  make logs   - Show container logs"
	@echo "  make shell  - Open a shell in the container"

build:
	docker-compose build

up:
	docker-compose up -d

down:
	docker-compose down

logs:
	docker-compose logs -f

shell:
	docker-compose exec serena /bin/bash
