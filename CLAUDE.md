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
| proxy (Rust) | `cargo build --release` | `cargo test` | `cargo run` |
| frontend (React/TS) | `npm run build` | `npm test` | `npm start` |
| node-ssh (Node.js) | `npm install` | — | `npm start` |
| terraform_provider (Go) | `go build -o terraform-provider-aws2 .` | `go test ./...` | — |

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
              ├── /api/*  → proxy (8080) → backend (8081)
              └── /       → static React build

Browser → React dev server (port 3000, local dev)
              └── direct → proxy (8080) → backend (8081)

Browser → node-ssh (port 3001, WebSocket)
              └── spawns ssh process via node-pty to VM SSH port
```

### Component Roles

- **backend** (`backend/`): Rust/Axum. Core VM API. Launches VMs with QEMU (copying `alpine.qcow2` as the base image), assigns a random SSH port (49152–65535), persists VM metadata as JSON files, and restarts all persisted VMs on startup.
- **proxy** (`proxy/`): Rust/Axum. Thin HTTP proxy that forwards all requests to the backend. Exists to provide a single network ingress point.
- **frontend** (`frontend/`): React 18 + TypeScript + Bootstrap 5 (CRA). Communicates with the proxy; in production uses relative `/api` path (nginx-proxied), in local dev targets `127.0.0.1:8080` via `.env.development`.
- **node-ssh** (`node-ssh/`): Node.js WebSocket server. Accepts WebSocket connections with `?host=&port=&user=` query params and spawns an SSH process via `node-pty`.
- **terraform_provider** (`terraform_provider/`): Go Terraform provider (`aws2`). Wraps the proxy API to manage VMs as Terraform resources. Defaults to `http://127.0.0.1:8080`.
- **RAG** (`RAG/`): Python RAG server. Deployed separately via Ansible; no local build step.

### VM Storage (backend)

VM metadata is stored as JSON files (`{uuid}.json`) in the directory configured by `storage.metadata_dir`. QCOW2 disk images are stored in `storage.qcow2_dir`. Both are configured in:

- `backend/config.toml` — local dev (uses `./vm-data`)
- `backend/config.ci.toml` — CI (uses `/tmp/vm-data`, selected when `CI` env var is set)
- Production config is generated inline by Ansible (not committed)

All `vm_db` functions take an explicit `dir: &Path` parameter rather than reading from global config — pass the path from `Config::load()`.

## Development Philosophy

This project follows **Test Driven Development (TDD)**:

- Write the test **before** the implementation
- No new functions, handlers, or modules without corresponding tests
- Tests live in a `#[cfg(test)]` block in the same file (Rust), alongside the source file (Go), or in a `*.test.ts` file (TypeScript)
- If a function is hard to test directly (e.g. requires root or external binaries), extract the pure logic into a helper and test that

TDD is not optional and should not need to be requested — it is the default way of working in this codebase.

## Deployment

Deploys to `pc1` (10.0.0.1) via Ansible. Run `make all` from the repo root. Requires SSH access with password (`--ask-pass`) and sudo (`-K`). The `aws_base_path` on the server is `/home/andy/aws`.
