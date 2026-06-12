# Orchestrator Service

A Rust/Axum service that routes and load-balances requests across registered backend workers, resolves VM IPs in bridge mode, and persists resource-to-backend mappings across restarts.

## Features

- **Load Balancing**: Round-robin distribution of VM launch requests across registered backends
- **Request Routing**: Routes VM/volume operations to the backend that owns each resource
- **MAC-to-IP Resolution**: Resolves VM IP addresses from the dnsmasq lease file (bridge mode)
- **State Persistence**: Survives restarts by persisting vm-backend and volume-backend mappings to disk
- **CORS Support**: Configurable CORS headers for cross-origin requests
- **Swagger UI**: OpenAPI documentation at `/swagger-ui`
- **Logging**: Structured logging with configurable log levels

## Configuration

The orchestrator service is configured using environment variables:

- `ORCHESTRATOR_PORT`: Port for the orchestrator service to listen on (default: `8080`)
- `LISTEN_IP`: IP address to bind to (default: `127.0.0.1`)
- `RUST_LOG`: Log level (default: `info`)
- `VM_BACKENDS_FILE`: Path to the vm-to-backend mapping file (default: `./vm-backends.json`)
- `VOLUME_BACKENDS_FILE`: Path to the volume-to-backend mapping file (default: `./volume-backends.json`)
- `LEASE_FILE`: Path to the dnsmasq lease file (default: `/var/lib/misc/dnsmasq.leases`)

## API Endpoints

- `POST /register` — Called by backend workers on startup to register themselves
- `POST /launch-vm` — Launch a new VM (round-robin across backends)
- `GET /list-vms` — Aggregated list of VMs from all backends
- `DELETE /delete-vm` — Delete a VM (routed to the owning backend)
- `POST /stop-vm` — Gracefully stop a running VM
- `POST /start-vm` — Resume a stopped VM
- `POST /launch-volume` — Create a new volume
- `GET /list-volumes` — Aggregated list of volumes from all backends
- `DELETE /delete-volume` — Delete a volume
- `GET /volume-files/:id` — List files in a volume

## Building and Running

### Prerequisites

- Rust 1.70+ and Cargo
- At least one backend worker running and able to call `/register`

### Build

```bash
cd orchestrator
cargo build --release
```

### Run

```bash
cargo run

# With custom configuration
ORCHESTRATOR_PORT=9090 cargo run

# Run the release binary
./target/release/orchestrator
```

## Architecture

The orchestrator service consists of:

- **Config Module**: Environment-based configuration
- **Registry**: Backend worker registry with round-robin load balancing and per-resource routing
- **Orchestrator Service**: HTTP forwarding logic and MAC-to-IP resolution
- **IP Lookup**: Resolves VM MAC addresses to IPs via dnsmasq lease file and ARP

## Request Flow

1. Backend workers POST to `/register` on startup
2. Client sends request to orchestrator
3. Orchestrator routes request to the appropriate backend worker
4. Backend processes request and returns response
5. Orchestrator forwards response back to client

## Development

```bash
cargo install cargo-watch
cargo watch -x run
```

## Testing

```bash
cargo test
```

## Logging

- `error`: Failed requests and system errors
- `info`: Request forwarding and server status
- `debug`: Detailed request/response information
