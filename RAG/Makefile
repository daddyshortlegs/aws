.PHONY: deploy all clean


deploy:
	ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook -i ../inventory/nodes.yaml deploy-rag.yaml -u andy -K --ask-pass

clean:
	rm -rf target

all: deploy
