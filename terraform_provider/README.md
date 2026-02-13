# Terraform Provider for VM Launcher (local use only)

This provider manages VMs via your [VM launcher proxy](../proxy) API. It is intended for **local use only** (no registry publish).

## Prerequisites

- [Go](https://go.dev/dl/) 1.21+
- [Terraform](https://www.terraform.io/downloads) 1.0+
- Proxy and backend running (proxy default: `http://127.0.0.1:8080`)

## Build the provider

From this directory (`terraform_provider/`):

```bash
go build -o terraform-provider-vm-launcher .
```

The binary must be named `terraform-provider-vm-launcher` (hyphen, to match the source type `vm-launcher`) and live in the directory you point Terraform at (see below).

## Use with Terraform (dev overrides)

1. **Build the provider** (see above).

2. **Configure Terraform to use the local provider** by copying `terraform.rc.example` to `~/.terraformrc` (or `%APPDATA%\terraform.rc` on Windows), then edit the path to the **absolute path** of the `terraform_provider` directory. The provider key must be in quotes:

   ```hcl
   provider_installation {
     dev_overrides {
       "localhost/myorg/vm-launcher" = "/Users/you/home-git/aws/terraform_provider"
     }
     direct {}
   }
   ```

   Alternatively set `TF_CLI_CONFIG_FILE` to the path of your copy of the example file.

3. **Run the proxy** (and backend) so the API is available at e.g. `http://127.0.0.1:8080`.

4. **From the examples directory:** When using dev_overrides, you can skip `terraform init` (Terraform will use the local binary directly). If you run `terraform init` anyway, clear cache first to avoid "hashicorp/vm" or localhost connection errors:

   ```bash
   cd examples
   rm -rf .terraform .terraform.lock.hcl
   terraform plan    # or terraform apply; init is optional with dev_overrides
   terraform apply
   ```

5. **Destroy the VM:**

   ```bash
   terraform destroy
   ```

## Provider configuration

| Argument         | Required | Default                 | Description                          |
|------------------|----------|-------------------------|--------------------------------------|
| `proxy_base_url` | No       | `http://127.0.0.1:8080` | Base URL of the VM launcher proxy.   |

## Resource: `vmlauncher_vm`

| Attribute        | Required | Computed | Description                          |
|------------------|----------|----------|--------------------------------------|
| `id`             | —        | yes      | Instance ID (UUID).                  |
| `name`           | yes      | —        | VM name.                             |
| `instance_type`  | no       | yes      | Instance type (default `t2.micro`).  |
| `region`         | no       | yes      | Region (default `us-east-1`).         |
| `ssh_port`       | —        | yes      | SSH port.                            |
| `pid`            | —        | yes      | Process ID on the host.              |

## Example

```hcl
terraform {
  required_providers {
    vmlauncher = {
      source  = "localhost/myorg/vm-launcher"
      version = "0.1.0"
    }
  }
}

provider "vmlauncher" {
  proxy_base_url = "http://127.0.0.1:8080"
}

resource "vmlauncher_vm" "my_vm" {
  name = "my-vm"
}
```

(Use `~/.terraformrc` with `dev_overrides` so Terraform finds the local provider binary; see `terraform.rc.example`.)

## Local use only

This provider is not published to the Terraform Registry. Terraform uses it via `dev_overrides` pointing at your local `terraform_provider` directory.
