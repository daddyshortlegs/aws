.PHONY: build audit deploy all

audit:
	npm audit --audit-level=high

build: audit
	npm run build

deploy:
	ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook -i ../inventory/nodes.yaml deploy-frontend.yaml -u andy -K --ask-pass

all: build deploy
