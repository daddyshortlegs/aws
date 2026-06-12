# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Andy's Web Services (AWS) — a virtual machine management system. Users can launch, list, and delete QEMU-based VMs via a web UI, and connect to them over SSH through a browser terminal.

## Build Commands

All components can be built from the repo root:

```bash
make build          # build all components natively (Mac dev/CI)
make build-linux    # cross-compile Rust binaries for Linux via podman (for deployment)
make audit          # run security audits across all components
make all            # build-linux + deploy (full deploy to pc1)
```

Per-component builds (run from the component directory or via root):

| Component | Build | Test | Run (dev) |
|-----------|-------|------|-----------|
| backend (Rust) | `cargo build --release` | `cargo test` | `cargo run` |
| orchestrator (Rust) | `cargo build --release` | `cargo test` | `cargo run` |
| frontend (React/TS) | `npm run build` | `npm test` | `npm start` |
| node-ssh (Node.js) | `npm install` | — | `npm start` |
| terraform_provider (Go) | `go build -o terraform-provider-aws2 .` | `go test ./...` | — |
| cli (Rust) | `cargo build --release` | `cargo test` | — |

**Run a single Rust test:**
```bash
cargo test test_name              # e.g. cargo test test_store_and_get_vm
cargo test test_name -- --nocapture  # with stdout
```

**Local dev (all services):**
```bash
make dev    # builds then starts all services via start.sh
make stop   # stop all services
```

Local dev starts **one orchestrator and three backends** (ports 8081, 8082, 8083) so the round-robin load balancing can be exercised. Backend instances are selected by the `APP_ENV` env var: unset or `config` → `config.toml`; `backend2` → `config.backend2.toml`; `backend3` → `config.backend3.toml`. When `CI` is set, `config.ci.toml` is used instead regardless of `APP_ENV`.

## Git Hooks (pre-commit)

Install once:
```bash
brew bundle
pre-commit install
```

Run against all files manually:
```bash
pre-commit run --all-files
```

Hooks enforce: trailing whitespace, EOF newline, valid YAML/TOML, `cargo fmt --check` + `cargo clippy -D warnings` (per Rust crate), `gofmt` + `go vet` (terraform_provider), ESLint (frontend/src).

**After editing any Rust file**, always run these before finishing:
```bash
cargo fmt --check
cargo clippy -- -D warnings
```
Fix any issues before stopping. Clippy warnings are treated as errors (`-D warnings`).

## Architecture

```
Browser → nginx (port 80, production)
              ├── /api/*  → orchestrator (8080) → backend (8081)
              └── /       → static React build

Browser → React dev server (port 3000, local dev)
              └── direct → orchestrator (8080) → backend (8081/8082/8083)

Browser → node-ssh (port 3001, WebSocket)
              └── spawns ssh process via node-pty to VM SSH port
```

### Component Roles

- **backend** (`backend/`): Rust/Axum. Core VM API. Launches VMs with QEMU (copying `alpine.qcow2` as the base image), assigns a random SSH port (49152–65535), persists VM metadata as JSON files, and restarts all persisted VMs on startup. On startup, self-registers with the orchestrator via `POST /register` so the orchestrator can route to it.
- **orchestrator** (`orchestrator/`): Rust/Axum. Routes and load-balances requests across registered backend workers; aggregates list responses from all backends. Resolves VM IPs from the dnsmasq lease file in bridge mode. Persists vm_id→backend and volume_id→backend mappings to `./vm-backends.json` and `./volume-backends.json` across restarts. Exposes Swagger UI at `/swagger-ui`.
- **frontend** (`frontend/`): React 18 + TypeScript + Bootstrap 5 (CRA). Communicates with the orchestrator; in production uses relative `/api` path (nginx-proxied), in local dev targets `127.0.0.1:8080` via `.env.development`.
- **node-ssh** (`node-ssh/`): Node.js WebSocket server. Accepts WebSocket connections with `?host=&port=&user=` query params and spawns an SSH process via `node-pty`.
- **cli** (`cli/`): Rust CLI (`andy-cli`). Wraps the orchestrator API for scripting. Uses `ANDYWS_ENDPOINT` env var (default `http://127.0.0.1:8080`) or `--orchestrator <url>`. Supports `vm list/launch/delete` and `volume list/launch/delete/files`.
- **terraform_provider** (`terraform_provider/`): Go Terraform provider (`aws2`). Wraps the orchestrator API to manage VMs as Terraform resources. Defaults to `http://127.0.0.1:8080`.
- **RAG** (`RAG/`): Python RAG server. Deployed separately via Ansible; no local build step.

### Storage (backend)

VM metadata is stored as JSON files (`{uuid}.json`), QCOW2 disk images are stored separately, and volumes (ext4 images mounted via loop device) are stored in a third directory. All three paths are configured under `[storage]` in the backend config:

- `backend/config.toml` — local dev instance 1 (ports 8081, vm-data/)
- `backend/config.backend2.toml` — local dev instance 2 (port 8082, vm-data-2/)
- `backend/config.backend3.toml` — local dev instance 3 (port 8083, vm-data-3/)
- `backend/config.ci.toml` — CI (uses `/tmp/vm-data`, selected when `CI` env var is set)
- Production config is generated inline by Ansible (not committed)

All `vm_db` and `volume_db` functions take an explicit `dir: &Path` parameter rather than reading from global config — pass the path from `Config::load()`.

### Orchestrator Configuration

The orchestrator is configured entirely via environment variables (no config file):

| Variable | Default | Purpose |
|---|---|---|
| `ORCHESTRATOR_PORT` | `8080` | Listening port |
| `LISTEN_IP` | `127.0.0.1` | Bind address |
| `RUST_LOG` | `info` | Log level |
| `VM_BACKENDS_FILE` | `./vm-backends.json` | vm_id→backend persistence |
| `VOLUME_BACKENDS_FILE` | `./volume-backends.json` | volume_id→backend persistence |
| `LEASE_FILE` | `/var/lib/misc/dnsmasq.leases` | dnsmasq lease file for bridge-mode IP lookup |

### Network Modes (backend)

Two QEMU networking modes are implemented. Set via `network_mode` in the backend config:

- **`user`** (default): QEMU user-mode networking with SSH port forwarding. SSH host is `127.0.0.1`, port is the randomly assigned port.
- **`bridge`**: Tap interface with a MAC address; DHCP via dnsmasq. The orchestrator resolves the VM's IP from the dnsmasq lease file when listing VMs.

## Development Philosophy

This project follows **Test Driven Development (TDD)**:

- Write the test **before** the implementation
- No new functions, handlers, or modules without corresponding tests
- Tests live in a `#[cfg(test)]` block in the same file (Rust), alongside the source file (Go), or in a `*.test.ts` file (TypeScript)
- If a function is hard to test directly (e.g. requires root or external binaries), extract the pure logic into a helper and test that

TDD is not optional and should not need to be requested — it is the default way of working in this codebase.

## Deployment

Deploys to a three-node cluster (`pc1` at 10.0.0.1, `pc2` at 10.0.0.2, `pc3` at 10.0.0.3) via Ansible. `pc1` is the controller (runs orchestrator + nginx); all three nodes run backend workers. Run `make all` from the repo root. Requires SSH access with password (`--ask-pass`) and sudo (`-K`). The `aws_base_path` on the server is `/home/andy/aws`.
