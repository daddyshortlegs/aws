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


