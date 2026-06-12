.PHONY: build build-backend build-orchestrator build-frontend build-node-ssh build-cli \
	build-linux build-linux-backend build-linux-orchestrator build-linux-cli \
	audit audit-backend audit-orchestrator audit-frontend audit-node-ssh audit-terraform audit-cli \
	dev run start stop \
	deploy deploy-backend deploy-orchestrator deploy-frontend deploy-node-ssh \
	install-node-exporter \
	all clean

# ── Audit ────────────────────────────────────────────────────────────────────
audit-backend:
	cd backend && $(MAKE) audit

audit-orchestrator:
	cd orchestrator && $(MAKE) audit

audit-frontend:
	cd frontend && $(MAKE) audit

audit-node-ssh:
	cd node-ssh && $(MAKE) audit

audit-terraform:
	cd terraform_provider && $(MAKE) audit

audit-cli:
	cd cli && $(MAKE) audit

audit: audit-backend audit-orchestrator audit-frontend audit-node-ssh audit-terraform audit-cli

# ── Local / CI native build ──────────────────────────────────────────────────
build-backend:
	cd backend && $(MAKE) build

build-orchestrator:
	cd orchestrator && $(MAKE) build

build-frontend:
	cd frontend && $(MAKE) build

build-node-ssh:
	cd node-ssh && $(MAKE) build

build-cli:
	cd cli && $(MAKE) build

build: build-backend build-orchestrator build-frontend build-node-ssh build-cli

# ── Linux cross-compile (for deployment from Mac) ────────────────────────────
build-linux-backend:
	cd backend && $(MAKE) build-linux

build-linux-orchestrator:
	cd orchestrator && $(MAKE) build-linux

build-linux-cli:
	cd cli && $(MAKE) build-linux

build-linux: build-linux-backend build-linux-orchestrator build-linux-cli build-frontend build-node-ssh

# ── Local dev: run all services ──────────────────────────────────────────────
start:
	./start.sh

stop:
	./stop.sh

dev: build start

# ── Deploy ───────────────────────────────────────────────────────────────────
deploy-backend:
	cd backend && $(MAKE) deploy

deploy-orchestrator:
	cd orchestrator && $(MAKE) deploy

deploy-frontend:
	cd frontend && $(MAKE) deploy

deploy-node-ssh:
	cd node-ssh && $(MAKE) deploy

deploy: deploy-backend deploy-orchestrator deploy-frontend deploy-node-ssh

# ── Build Linux binaries and deploy ──────────────────────────────────────────
all: build-linux deploy

# ── Infrastructure setup (run once per host) ─────────────────────────────────
install-node-exporter:
	ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook -i inventory/nodes.yaml ansible/install-node-exporter.yaml -u andy -K --ask-pass

# ── Clean ────────────────────────────────────────────────────────────────────
clean:
	cd backend && $(MAKE) clean 2>/dev/null || true
	cd orchestrator && $(MAKE) clean 2>/dev/null || true
	cd cli && $(MAKE) clean 2>/dev/null || true
	cd frontend && $(MAKE) clean 2>/dev/null || true
	cd node-ssh && $(MAKE) clean 2>/dev/null || true
