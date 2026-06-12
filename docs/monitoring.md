# Monitoring

The monitoring stack is Prometheus + Grafana, with node_exporter running on each cluster node.

## Components

| Component | Host | Port | Purpose |
|-----------|------|------|---------|
| node_exporter | 10.0.0.1, 10.0.0.2, 10.0.0.3 | 9100 | Exposes host metrics (CPU, memory, disk, network) |
| Prometheus | 10.0.0.1 (controller) | 9090 | Scrapes node_exporter on all nodes every 15s |
| Grafana | 10.0.0.1 (controller) | 3100 | Dashboards (port 3000 is taken by the frontend) |

## URLs

- **Grafana**: http://10.0.0.1:3100 — default login `admin` / `admin`, change on first login
- **Prometheus**: http://10.0.0.1:9090

## Installation

Run from the repo root. Each playbook requires sudo on the target host (`-K` prompts for the password):

```bash
# Install node_exporter on all cluster nodes
make install-node-exporter

# Install Prometheus on the controller
make install-prometheus

# Install Grafana on the controller
make install-grafana
```

## Connecting Grafana to Prometheus

After installing, add Prometheus as a data source in Grafana:

1. Open Grafana at http://10.0.0.1:3100
2. Go to **Connections → Data sources → Add data source**
3. Choose **Prometheus**
4. Set the URL to `http://localhost:9090`
5. Click **Save & test**

## Prometheus Scrape Targets

Prometheus is configured to scrape node_exporter from all three cluster nodes:

- `10.0.0.1:9100`
- `10.0.0.2:9100`
- `10.0.0.3:9100`

Config lives at `/etc/prometheus/prometheus.yml` on the controller. Re-run `make install-prometheus` to push changes.
