# Andy's Web Services

To run:

```
cargo run
```

To launch a VM:

```
curl -X POST http://localhost:3000/launch-vm -H "Content-Type: application/json" -d '{"name": "test-vm", "instance_type": "t2.micro", "region": "us-west-2"}'
```
