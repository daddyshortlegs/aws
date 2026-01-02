export ANSIBLE_HOST_KEY_CHECKING=False
ansible-playbook -i ../inventory/nodes.yaml deploy-frontend.yaml -u andy -K --ask-pass
