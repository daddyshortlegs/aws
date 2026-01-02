.PHONY: deploy all clean

deploy:
	ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook -i ../inventory/nodes.yaml deploy-node-ssh.yaml -u andy -K --ask-pass

clean:
	rm -rf $(BUILD_DIR)

all: build deploy

