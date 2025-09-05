# Andy's Web Services

To run:

```
cargo run
```

To launch a VM:

```
curl -X POST http://localhost:8080/launch-vm -H "Content-Type: application/json" -d '{"name": "test-vm", "instance_type": "t2.micro", "region": "us-west-2"}'
```

To list VMs:

```
curl http://localhost:8080/list-vms
```

To delete a VM:

```
curl -X DELETE http://localhost:8080/delete-vm -H "Content-Type: application/json" -d '{"id": "3418ca7b-4148-473b-b897-81a11f2dccfa"}'
```




## Notes

Needs to be base image already installed with Ubuntu. When firing up new VM, we'd need a new copy of it so any changes made to it are specific to whoever started it.
