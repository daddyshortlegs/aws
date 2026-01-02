.PHONY: build deploy all

# Build target - build in Debian Linux container (x86_64)
build:
	mkdir -p target
	podman run --rm --platform linux/amd64 \
		-v "$(PWD):/workspace" \
		-v "$(PWD)/target:/workspace/target" \
		-w /workspace \
		rust:1-bookworm \
		cargo build --release

deploy:
	ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook -i ../inventory/nodes.yaml deploy-proxy.yaml -u andy -K --ask-pass

all: build deploy

