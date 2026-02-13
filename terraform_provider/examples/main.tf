# Use the local VM launcher provider via dev_overrides (no registry).
# Run from this directory: terraform init && terraform plan && terraform apply
#
# Prerequisites:
# - Proxy running (e.g. http://127.0.0.1:8080)
# - Provider built: from terraform_provider/ run: go build -o terraform-provider-vm-launcher .
# - CLI config: copy terraform_provider/terraform.rc.example to ~/.terraformrc
#   (or set TF_CLI_CONFIG_FILE to its path) so Terraform finds the local provider.

terraform {
  required_providers {
    vmlauncher = {
      source  = "localhost/myorg/vm-launcher"
      version = "0.1.0"
    }
  }
}

provider "vmlauncher" {
  # Optional: proxy URL (defaults to http://127.0.0.1:8080)
  proxy_base_url = "http://127.0.0.1:8080"
}

resource "vmlauncher_vm" "andy1" {
  name = "andy1-vm"
  # instance_type = "t2.micro"   # optional, default
  # region       = "us-east-1"    # optional, default
}

resource "vmlauncher_vm" "andy2" {
  name = "andy2-vm"
  # instance_type = "t2.micro"   # optional, default
  # region       = "us-east-1"    # optional, default
}

output "vm_id_1" {
  value       =  vmlauncher_vm.andy1.id
  description = "Instance ID of VM 1"
}

output "ssh_port_1" {
  value       = vmlauncher_vm.andy1.ssh_port
  description = "SSH port for VM 1"
}

output "pid_1" {
  value       = vmlauncher_vm.andy1.pid
  description = "Process ID of VM 1"
}

output "vm_id_2" {
  value       =  vmlauncher_vm.andy2.id
  description = "Instance ID of VM 2"
}

output "ssh_port_2" {
  value       = vmlauncher_vm.andy2.ssh_port
  description = "SSH port for VM 2"
}

output "pid_2" {
  value       = vmlauncher_vm.andy2.pid
  description = "Process ID of VM 2"
}
