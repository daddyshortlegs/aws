# Epics

- [X] Node controller with API to register
- [X] Terraform provider
- [ ] RAG to query APIs
- [ ] Metrics and monitoring with Prometheus
- [ ] S3 style buckets
- [ ] Kubernetes deployment on VMs

# Tasks

- [X] Change frontend and backend to work off IP addresses for the VMs, rather than port numbers
- [ ] UI shows "Waiting for IP..." even after the VM has received an IP from dnsmasq — list-vms is not returning the IP correctly in production
- [ ] Connect button in the UI does nothing — needs investigation (node-ssh WebSocket connection may be failing)
- [X] Ansible scripts must install qemu-system-x86 before the backend can work
- [X] Front end cant connect to proxy
- [X] The frontend app hardcodes the domain to 127.0.0.1, which results in a CORS error when it tries to make an API request
- [ ] If the qcow2 file doesn't exist for a particular OS, the server returns a 500 Internal Server Error, rather than a helpful log message or response.
