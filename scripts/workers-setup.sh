#!/usr/bin/env bash
# Setup script for PC2 (10.0.0.2) and PC3 (10.0.0.3) — Worker Nodes with bridge networking
# Run as root or with sudo on a fresh Debian 12 (Bookworm) install.
#
# Usage:
#   sudo bash workers-setup.sh 2    # configure as 10.0.0.2
#   sudo bash workers-setup.sh 3    # configure as 10.0.0.3
set -euo pipefail

# ---------------------------------------------------------------------------
# Argument handling
# ---------------------------------------------------------------------------

if [[ $# -ne 1 || ( "$1" != "2" && "$1" != "3" ) ]]; then
    echo "Usage: $0 <2|3>"
    echo "  2 — configure this machine as PC2 (10.0.0.2)"
    echo "  3 — configure this machine as PC3 (10.0.0.3)"
    exit 1
fi

HOST_IP="10.0.0.$1"

echo "Configuring worker node as ${HOST_IP}..."

# ---------------------------------------------------------------------------
# Packages
# ---------------------------------------------------------------------------

apt-get install -y bridge-utils

# ---------------------------------------------------------------------------
# Network interfaces
# ---------------------------------------------------------------------------

cat > /etc/network/interfaces << EOF
auto lo
iface lo inet loopback

# Bridge for VMs on this host
auto br0
iface br0 inet static
    address ${HOST_IP}
    netmask 255.255.255.0
    gateway 10.0.0.1
    bridge_ports enp1s0
    bridge_stp off
    bridge_fd 0
    dns-nameservers 8.8.8.8 8.8.4.4
EOF

# ---------------------------------------------------------------------------
# QEMU TAP interface scripts
# ---------------------------------------------------------------------------

cat > /etc/qemu-ifup << 'EOF'
#!/bin/sh
# Bring up a TAP interface and add it to the bridge.
set -e

if [ -n "$1" ]; then
    ip link set "$1" up promisc on
    brctl addif br0 "$1"
    exit 0
else
    echo "Error: no interface specified"
    exit 1
fi
EOF
chmod +x /etc/qemu-ifup

cat > /etc/qemu-ifdown << 'EOF'
#!/bin/sh
# Remove a TAP interface from the bridge and bring it down.
set -e

if [ -n "$1" ]; then
    brctl delif br0 "$1"
    ip link set "$1" down
    exit 0
else
    echo "Error: no interface specified"
    exit 1
fi
EOF
chmod +x /etc/qemu-ifdown

# ---------------------------------------------------------------------------
# Apply network configuration
# ---------------------------------------------------------------------------

systemctl restart networking

echo "Worker node setup complete (${HOST_IP}). Verify with: brctl show && ip route && ping 10.0.0.1"
