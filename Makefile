.PHONY: build build-backend build-proxy build-frontend build-node-ssh \
	build-linux build-linux-backend build-linux-proxy \
	audit audit-backend audit-proxy audit-frontend audit-node-ssh audit-terraform \
	dev run start stop \
	deploy deploy-backend deploy-proxy deploy-frontend deploy-node-ssh \
	all clean

# ── Audit ────────────────────────────────────────────────────────────────────
audit-backend:
	cd backend && $(MAKE) audit

audit-proxy:
	cd proxy && $(MAKE) audit

audit-frontend:
	cd frontend && $(MAKE) audit

audit-node-ssh:
	cd node-ssh && $(MAKE) audit

audit-terraform:
	cd terraform_provider && $(MAKE) audit

audit: audit-backend audit-proxy audit-frontend audit-node-ssh audit-terraform

# ── Local / CI native build ──────────────────────────────────────────────────
build-backend:
	cd backend && $(MAKE) build

build-proxy:
	cd proxy && $(MAKE) build

build-frontend:
	cd frontend && $(MAKE) build

build-node-ssh:
	cd node-ssh && $(MAKE) build

build: build-backend build-proxy build-frontend build-node-ssh

# ── Linux cross-compile (for deployment from Mac) ────────────────────────────
build-linux-backend:
	cd backend && $(MAKE) build-linux

build-linux-proxy:
	cd proxy && $(MAKE) build-linux

build-linux: build-linux-backend build-linux-proxy build-frontend build-node-ssh

# ── Local dev: run all services ──────────────────────────────────────────────
start:
	./start.sh

stop:
	./stop.sh

dev: build start

# ── Deploy ───────────────────────────────────────────────────────────────────
deploy-backend:
	cd backend && $(MAKE) deploy

deploy-proxy:
	cd proxy && $(MAKE) deploy

deploy-frontend:
	cd frontend && $(MAKE) deploy

deploy-node-ssh:
	cd node-ssh && $(MAKE) deploy

deploy: deploy-backend deploy-proxy deploy-frontend deploy-node-ssh

# ── Build Linux binaries and deploy ──────────────────────────────────────────
all: build-linux deploy

# ── Clean ────────────────────────────────────────────────────────────────────
clean:
	cd backend && $(MAKE) clean 2>/dev/null || true
	cd proxy && $(MAKE) clean 2>/dev/null || true
	cd frontend && $(MAKE) clean 2>/dev/null || true
	cd node-ssh && $(MAKE) clean 2>/dev/null || true
