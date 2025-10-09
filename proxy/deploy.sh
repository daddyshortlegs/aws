export ANSIBLE_HOST_KEY_CHECKING=False
cargo build --release
ansible-playbook -i ../inventory/nodes.yaml deploy-proxy.yaml -u andy -K --ask-pass
