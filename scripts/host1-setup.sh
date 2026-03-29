#!/usr/bin/env bash
# Setup script for PC1 (10.0.0.1) — Gateway/Router with bridge networking
# Run as root or with sudo on a fresh Debian 12 (Bookworm) install.
set -euo pipefail

# ---------------------------------------------------------------------------
# Packages
# ---------------------------------------------------------------------------

apt-get install -y bridge-utils

# ---------------------------------------------------------------------------
# Network interfaces
# NOTE: wlx7c3d09636a60 is the WAN (WiFi) interface name — update if yours
#       differs. wpa-psk is stored in plaintext here; consider wpa_supplicant
#       or NetworkManager for more secure credential handling.
# ---------------------------------------------------------------------------

cat > /etc/network/interfaces << 'EOF'
source /etc/network/interfaces/d/*

auto lo
iface lo inet loopback

# Bridge for internal network (VMs + physical hosts)
auto br0
iface br0 inet static
    address 10.0.0.1
    netmask 255.255.255.0
    bridge_ports enp1s0
    bridge_stp off
    bridge_fd 0

# External/WAN interface
auto wlx7c3d09636a60
iface wlx7c3d09636a60 inet dhcp
    wpa-ssid blah
    wpa-psk  secret
EOF

# ---------------------------------------------------------------------------
# IP forwarding
# ---------------------------------------------------------------------------

cat > /etc/sysctl.d/99-ip-forward.conf << 'EOF'
net.ipv4.ip_forward=1
EOF

sysctl -w net.ipv4.ip_forward=1

# ---------------------------------------------------------------------------
# NAT/Masquerading via nftables
# NOTE: oifname must match the WAN interface — wlx7c3d09636a60 here.
# ---------------------------------------------------------------------------

cat > /etc/nftables.conf << 'EOF'
#!/usr/sbin/nft -f

flush ruleset

table ip nat {
    chain postrouting {
        type nat hook postrouting priority 100;
        oifname "wlx7c3d09636a60" masquerade
    }
}

table ip filter {
    chain forward {
        type filter hook forward priority 0; policy drop;
        iifname "br0" oifname "wlx7c3d09636a60" accept
        iifname "wlx7c3d09636a60" oifname "br0" ct state related,established accept
    }
}
EOF

systemctl enable --now nftables

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

echo "PC1 setup complete. Verify with: brctl show && sysctl net.ipv4.ip_forward && sudo nft list ruleset"
