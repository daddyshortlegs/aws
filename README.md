# Andy's Web Services

To run:

```
cargo run
```

To launch a VM:

```
curl -X POST http://localhost:3000/launch-vm -H "Content-Type: application/json" -d '{"name": "test-vm", "instance_type": "t2.micro", "region": "us-west-2"}'
```



## QEMU stuff

Create a virtual disk:

```
qemu-img create -f qcow2 ubuntu.qcow2 20G
```

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



## Notes

Needs to be base image already installed with Ubuntu. When firing up new VM, we'd need a new copy of it so any changes made to it are specific to whoever started it.
