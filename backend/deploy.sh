export ANSIBLE_HOST_KEY_CHECKING=False
cargo build --target x86_64-unknown-linux-gnu --release
ansible-playbook -i ../inventory/nodes.yaml deploy-backend.yaml -u andy -K --ask-pass
