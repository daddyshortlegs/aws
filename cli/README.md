# andy-cli

Command-line client for Andy's Web Services. Communicates with the proxy.

## Build

```bash
cargo build --release
```

The binary is at `target/release/andy-cli`.

## Configuration

| Variable           | Description                  | Default                   |
|--------------------|------------------------------|---------------------------|
| `ANDYWS_ENDPOINT`  | Proxy base URL               | `http://127.0.0.1:8080`   |

You can also pass `--proxy <url>` directly on the command line, which takes precedence over the environment variable.

## Usage

```bash
# Use the environment variable
export ANDYWS_ENDPOINT=http://10.0.0.1:8080

andy-cli vm list
andy-cli vm launch --name my-vm
andy-cli vm delete --id <id>

andy-cli volume list
andy-cli volume launch --name my-data --size-gb 10
andy-cli volume delete --id <id>
andy-cli volume files --id <id>
```

Add `--json` to any command to get raw JSON output instead of a formatted table:

```bash
andy-cli --json vm list
```
