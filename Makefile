.PHONY: build-backend build-proxy build-frontend build-node-ssh \
	deploy-backend deploy-proxy deploy-frontend deploy-node-ssh \
	build deploy all clean

# Build targets for individual services
build-backend:
	cd backend && $(MAKE) build

build-proxy:
	cd proxy && $(MAKE) build

build-frontend:
	cd frontend && $(MAKE) build

build-node-ssh:
	cd node-ssh && $(MAKE) build

# Deploy targets for individual services
deploy-backend:
	cd backend && $(MAKE) deploy

deploy-proxy:
	cd proxy && $(MAKE) deploy

deploy-frontend:
	cd frontend && $(MAKE) deploy

deploy-node-ssh:
	cd node-ssh && $(MAKE) deploy

build: build-backend build-proxy build-frontend build-node-ssh

deploy: deploy-backend deploy-proxy deploy-frontend deploy-node-ssh

# Build and deploy all services
all: build deploy

# Clean all build artifacts
clean:
	cd backend && $(MAKE) clean 2>/dev/null || true
	cd proxy && $(MAKE) clean 2>/dev/null || true
	cd frontend && $(MAKE) clean 2>/dev/null || true
	cd node-ssh && $(MAKE) clean 2>/dev/null || true

