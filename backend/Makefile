.PHONY: build build-linux run audit deploy all clean

audit:
	cargo audit

# Native build — works on Mac (dev/test) and Linux (CI)
build: audit
	cargo build --release

# Cross-compile for Linux x86_64 via container — use when deploying from Mac
build-linux:
	mkdir -p target
	podman run --rm --platform linux/amd64 \
		-v "$(PWD):/workspace" \
		-v "$(PWD)/target:/workspace/target" \
		-w /workspace \
		rust:1-bookworm \
		cargo build --release

run:
	cargo run

deploy: build-linux
	ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook -i ../inventory/nodes.yaml deploy-backend.yaml -u andy -K --ask-pass

clean:
	rm -rf target

all: build-linux deploy
