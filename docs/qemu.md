## QEMU

### Installing an OS

Create a virtual disk:

```
qemu-img create -f qcow2 ubuntu.qcow2 20G
```

Grab the List ISO from https://releases.ubuntu.com/noble/ubuntu-24.04.2-live-server-amd64.iso
Run the Live ISO image to install it to the 'disk' just created:

```
qemu-system-x86_64 -m 4096 -smp 2 -cdrom ubuntu-24.04.2-live-server-amd64.iso -drive file=ubuntu.qcow2 -boot d -vga virtio -net nic -net user
```

When installed onto the qcow2 disk, boot it:

```
qemu-system-x86_64 -m 4096 -smp 2 -drive file=ubuntu.qcow2 -boot d -vga virtio -netdev user,id=net0,hostfwd=tcp::2222-:22 -device e1000,netdev=net0
```

This sets up port forwarding so you can ssh into the machine:

```
ssh -P 2222 andy@localhost
```

### Monitoring

You can setup a QEMU monitor that you can issue commands to, add the `monitor` flag to qemu, e.g:

```
qemu-system-x86_64 -m 4096 -smp 2 -drive file=test-vm.qcow2 -boot d -vga virtio -netdev user,id=net0,hostfwd=tcp::2222-:22 -device e1000,netdev=net0 -monitor unix:/tmp/qemu-monitor.sock,server,nowait
```

You can connect to this socket with netcat:

```
nc -U /tmp/qemu-monitor.sock
```

Once connected you can issue commands, e.g. `system_powerdown`

### Bridged Networking

By default, QEMU uses **user-mode (SLIRP) networking** with port forwarding. This means VMs are not visible on the LAN — SSH access goes via a random port on the host (e.g. `ssh -p 54321 user@10.0.0.1`).

**Bridged networking** gives each VM its own IP address on the same LAN as the host, so you can SSH directly on port 22.

#### How it works

A bridge interface (`br0`) is created on the host. The physical NIC is attached to the bridge, and the host's IP moves to the bridge. Each VM connects via a TAP interface also attached to the bridge — making it appear as a real device on the network.

```
Physical LAN (10.0.0.x)
       |
   [br0 on host]  ← host IP lives here
   /     \
[eth0]  [tap0] [tap1]
              |       |
            [VM1]   [VM2]
```

#### Host setup (Ubuntu, Netplan)

Check your physical NIC name first: `ip link`

Edit `/etc/netplan/01-netcfg.yaml`:

```yaml
network:
  version: 2
  ethernets:
    ens3:           # your physical NIC
      dhcp4: false
  bridges:
    br0:
      interfaces: [ens3]
      addresses: [10.0.0.1/24]
      gateway4: 10.0.0.x
      nameservers:
        addresses: [8.8.8.8]
      parameters:
        stp: false
        forward-delay: 0
```

```bash
sudo netplan apply
```

> Do this over a console session, not SSH — the network drops briefly.

Allow QEMU's bridge helper to use `br0`:

```bash
sudo mkdir -p /etc/qemu
echo "allow br0" | sudo tee /etc/qemu/bridge.conf
sudo chmod 640 /etc/qemu/bridge.conf
sudo chown root:kvm /etc/qemu/bridge.conf

# Ensure the helper is setuid root:
sudo chmod u+s /usr/lib/qemu/qemu-bridge-helper
```

The user on the host needs to belong to the `kvm` group.
ufw needs a line adding to it and restarting:
# /etc/ufw/sysctl.conf — ensure bridge forwarding is on:
net/bridge/bridge-nf-call-iptables=0

Restart with sudo systemctl restart ufw

To check network for other hosts:

`nmap -sn 10.0.0.0/24`


#### Launching a VM with bridged networking

Replace the `user` netdev with `bridge`:

```bash
qemu-system-x86_64 \
  -m 4096 -smp 2 \
  -drive file=alpine.qcow2 \
  -netdev bridge,id=net0,br=br0 \
  -device e1000,netdev=net0 \
  -nographic
```

The VM will request a DHCP address from your router and appear on the LAN with its own IP.

#### Discovering a VM's IP address

QEMU doesn't tell you what IP the VM received — you have to discover it. Three approaches:

**1. Poll the ARP table (simplest, no image changes needed)**

Assign each VM a deterministic MAC address at launch (e.g. derived from its UUID), then poll the host's ARP table until the entry appears:

```bash
# Launch with a fixed MAC:
-netdev bridge,id=net0,br=br0 -device e1000,netdev=net0,mac=52:54:00:ab:cd:ef

# Poll for it after boot:
ip neigh show dev br0 | grep "52:54:00:ab:cd:ef"
# 10.0.0.15 dev br0 lladdr 52:54:00:ab:cd:ef REACHABLE
```

ARP entries expire when idle, so this is best used as a boot-time discovery mechanism and the result stored in the VM's metadata.

**2. DHCP leases (reliable if you control the DHCP server)**

If running `dnsmasq` on the host, check its leases file:

```bash
cat /var/lib/misc/dnsmasq.leases
# 1234567890 52:54:00:ab:cd:ef 10.0.0.15 alpine-vm *
```

Correlate by MAC address (same deterministic MAC approach as above).

**3. QEMU Guest Agent (most robust, requires image change)**

Install `qemu-guest-agent` inside the Alpine VM (`apk add qemu-guest-agent`). Launch QEMU with a virtio serial channel:

```bash
qemu-system-x86_64 \
  ... \
  -chardev socket,path=/tmp/qga-{uuid}.sock,server=on,wait=off,id=qga0 \
  -device virtio-serial \
  -device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0
```

Query the VM's IP directly from the host:

```bash
echo '{"execute":"guest-network-get-interfaces"}' \
  | socat - UNIX-CONNECT:/tmp/qga-{uuid}.sock
```

#### Recommended approach for this project

1. Derive a **deterministic MAC address** from the VM's UUID at launch time
2. After launch, **poll `ip neigh show dev br0`** for that MAC in a retry loop
3. Once found, **store the IP in the VM's JSON metadata**
4. Expose the VM's IP (rather than a host port) via the API
