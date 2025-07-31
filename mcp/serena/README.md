# Serena MCP Server

This directory contains the Docker setup for running Serena as an MCP (Model Context Protocol) server.

## Overview

Serena is a code AI agent that provides powerful code navigation and editing capabilities through the MCP protocol.

## Prerequisites

- Docker and Docker Compose installed
- Access to `ghcr.io/oraios/serena` Docker image

## Usage

### Start the server

```bash
make up
```

### View logs

```bash
make logs
```

### Stop the server

```bash
make down
```

### Access the container shell

```bash
make shell
```

## Configuration

The Docker setup mounts the project directory at `/workspace` inside the container, allowing Serena to access and modify your codebase.

## MCP Integration

To use this server with Claude or other MCP-compatible clients, configure your `.mcp.json` file to connect to this Docker container.

## References

- [Serena GitHub Repository](https://github.com/oraios/serena)
- [MCP Documentation](https://modelcontextprotocol.io/)
