.PHONY: build run audit deploy all clean

BUILD_DIR := build

audit:
	npm audit --audit-level=high

build: audit
	npm install

run:
	npm start

deploy: build
	ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook -i ../inventory/nodes.yaml deploy-node-ssh.yaml -u andy -K --ask-pass

clean:
	rm -rf $(BUILD_DIR)

all: build deploy
