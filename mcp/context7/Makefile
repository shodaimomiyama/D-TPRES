.PHONY: start stop build check clean help up down logs status

# Default target
help:
	@echo "Context7 MCP Server - Available commands:"
	@echo "  make build    - Build the Docker image"
	@echo "  make start    - Start the Context7 MCP server in Docker (foreground)"
	@echo "  make up       - Start the server in background using docker-compose"
	@echo "  make down     - Stop the background server"
	@echo "  make logs     - Show server logs"
	@echo "  make status   - Show server status"
	@echo "  make check    - Check prerequisites (Docker)"
	@echo "  make clean    - Clean up Docker images and containers"
	@echo "  make help     - Show this help message"

# Docker image name
IMAGE_NAME := context7-mcp

# Build the Docker image
build:
	@echo "Building Context7 MCP Docker image..."
	@docker build -t $(IMAGE_NAME) .
	@echo "Docker image built successfully: $(IMAGE_NAME)"

# Start the Context7 MCP server via Docker
start: check
	@echo "Starting Context7 MCP server with Docker..."
	@docker run -i --rm $(IMAGE_NAME)

# Check prerequisites
check:
	@echo "Checking prerequisites..."
	@echo "Checking Docker installation..."
	@docker --version || (echo "ERROR: Docker not found. Install Docker Desktop" && exit 1)
	@echo "Checking if Docker is running..."
	@docker info >/dev/null 2>&1 || (echo "ERROR: Docker is not running. Start Docker Desktop" && exit 1)
	@echo "All checks completed."

# Clean up Docker resources
clean:
	@echo "Cleaning up Docker resources..."
	@echo "Removing Docker image: $(IMAGE_NAME)"
	@docker rmi $(IMAGE_NAME) 2>/dev/null || echo "Image $(IMAGE_NAME) not found"
	@echo "Stopping any running containers..."
	@docker ps -q --filter ancestor=$(IMAGE_NAME) | xargs -r docker stop
	@echo "Cleanup completed."

# Development: rebuild and start
restart: clean build start

# Show Docker image info
info:
	@echo "Docker image information:"
	@docker images $(IMAGE_NAME) || echo "Image not built yet. Run 'make build' first."

# Start server in background using docker-compose
up: check
	@echo "Starting Context7 MCP server in background..."
	@docker-compose up -d --build
	@echo "Server started. Use 'make logs' to view logs or 'make down' to stop."

# Stop background server
down:
	@echo "Stopping Context7 MCP server..."
	@docker-compose down
	@echo "Server stopped."

# Show server logs
logs:
	@docker-compose logs -f

# Show server status
status:
	@echo "Context7 MCP server status:"
	@docker-compose ps
