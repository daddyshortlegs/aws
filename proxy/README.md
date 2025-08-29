# Proxy Service

A Rust-based HTTP proxy service built with Axum that forwards API requests to the backend VM launcher service.

## Features

- **HTTP Request Proxying**: Forwards all HTTP requests to the backend service
- **WebSocket Support**: Handles WebSocket connections (currently redirects to backend)
- **CORS Support**: Configurable CORS headers for cross-origin requests
- **Environment Configuration**: Configurable via environment variables
- **Logging**: Structured logging with configurable log levels

## Configuration

The proxy service can be configured using environment variables:

- `BACKEND_URL`: URL of the backend service (default: `http://127.0.0.1:8080`)
- `PROXY_PORT`: Port for the proxy service to listen on (default: `3000`)
- `RUST_LOG`: Log level (default: `info`)

## API Endpoints

The proxy forwards all requests to the corresponding backend endpoints:

- `POST /launch-vm` - Launch a new VM
- `GET /list-vms` - List all VMs
- `DELETE /delete-vm` - Delete a VM
- `GET /ws` - WebSocket connection for VM management

## Building and Running

### Prerequisites

- Rust 1.70+ and Cargo
- Backend service running on the configured backend URL

### Build

```bash
cd proxy
cargo build --release
```

### Run

```bash
# Using default configuration
cargo run

# With custom configuration
BACKEND_URL=http://localhost:8080 PROXY_PORT=3001 cargo run

# Run the release binary
./target/release/proxy
```

## Architecture

The proxy service consists of:

- **Config Module**: Handles environment-based configuration
- **Proxy Service**: Core logic for forwarding requests to the backend
- **Main Application**: Axum router setup and server initialization

## Request Flow

1. Client sends request to proxy service
2. Proxy service forwards request to backend service
3. Backend service processes request and returns response
4. Proxy service forwards response back to client

## Error Handling

- **Backend Unavailable**: Returns 502 Bad Gateway with error details
- **Invalid Requests**: Forwards backend error responses
- **Network Issues**: Logs errors and returns appropriate HTTP status codes

## Development

To run in development mode with hot reloading:

```bash
cargo install cargo-watch
cargo watch -x run
```

## Testing

```bash
cargo test
```

## Logging

The service uses structured logging with different levels:

- `error`: Failed requests and system errors
- `info`: Request proxying and server status
- `debug`: Detailed request/response information
- `trace`: Verbose debugging information
