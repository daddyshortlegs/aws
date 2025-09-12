# libssl-dev needed for cargo build and reqwest
sudo apt update && sudo apt install curl qemu-systemi libssl-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
# Need to source reload the terminal as next command doesn't work
nvm install node

