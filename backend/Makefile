.PHONY: build deploy all

# Build target - cross-compile for Linux x86_64
build:
	cargo build --target x86_64-unknown-linux-gnu --release

deploy:
	ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook -i ../inventory/nodes.yaml deploy-backend.yaml -u andy -K --ask-pass

all: build deploy

