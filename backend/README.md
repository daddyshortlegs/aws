# Andy's Web Services

To run:

```
cargo run
```

another instance:

```
APP_ENV=backend2 cargo run
```

where backend2 refers to a TOML file called config.backend2.toml.


## Virtual machines

To launch a VM:

```
curl -X POST http://localhost:8081/launch-vm -H "Content-Type: application/json" -d '{"name": "test-vm", "instance_type": "t2.micro", "region": "us-west-2"}'
```

To list VMs:

```
curl http://localhost:8081/list-vms
```

To delete a VM:

```
curl -X DELETE http://localhost:8081/delete-vm -H "Content-Type: application/json" -d '{"id": "3418ca7b-4148-473b-b897-81a11f2dccfa"}'
```


## Volumes

To list volumes:

```
curl http://localhost:8081/list-volumes
```

To create a volume:

```
curl -X POST http://10.0.0.1:8081/launch-volume -H "Content-Type: application/json" -d '{"name": "test-volume" ,"size_gb": 10}'
```

Delete a volume:

```
curl -X DELETE http://10.0.0.1:8081/delete-volume -H "Content-Type: application/json" -d '{"id": "e8bb6971-e57e-4263-8e3b-e554926fcfe0"}'
```

## Notes

Needs to be base image already installed with Ubuntu. When firing up new VM, we'd need a new copy of it so any changes made to it are specific to whoever started it.
