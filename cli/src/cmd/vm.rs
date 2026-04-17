use crate::client::Client;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand)]
pub enum VmCommand {
    /// Launch a new VM
    Launch {
        /// VM name
        #[arg(long)]
        name: String,
        /// Instance type (e.g. t2.micro)
        #[arg(long, default_value = "t2.micro")]
        instance_type: String,
        /// Region
        #[arg(long, default_value = "us-east-1")]
        region: String,
    },
    /// List all VMs
    List,
    /// Delete a VM
    Delete {
        /// VM ID
        #[arg(long)]
        id: String,
    },
}

#[derive(Serialize)]
struct LaunchVmRequest {
    name: String,
    instance_type: String,
    region: String,
}

#[derive(Deserialize)]
struct LaunchVmResponse {
    success: bool,
    message: String,
    instance_id: Option<String>,
    ssh_host: Option<String>,
    ssh_port: Option<u16>,
    pid: Option<u32>,
}

#[derive(Deserialize, Serialize)]
struct VmListEntry {
    id: String,
    name: String,
    ssh_host: String,
    ssh_port: u16,
    pid: u32,
}

#[derive(Serialize)]
struct DeleteVmRequest {
    id: String,
}

pub async fn run(cmd: VmCommand, client: &Client, json: bool) -> Result<(), String> {
    match cmd {
        VmCommand::Launch {
            name,
            instance_type,
            region,
        } => {
            let resp: LaunchVmResponse = client
                .post(
                    "/launch-vm",
                    &LaunchVmRequest {
                        name,
                        instance_type,
                        region,
                    },
                )
                .await?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": resp.success,
                        "message": resp.message,
                        "instance_id": resp.instance_id,
                        "ssh_host": resp.ssh_host,
                        "ssh_port": resp.ssh_port,
                        "pid": resp.pid,
                    }))
                    .unwrap()
                );
            } else if resp.success {
                println!("Launched VM");
                if let Some(id) = &resp.instance_id {
                    println!("  ID:       {id}");
                }
                if let (Some(host), Some(port)) = (&resp.ssh_host, resp.ssh_port) {
                    println!("  SSH:      {host}:{port}");
                }
                if let Some(pid) = resp.pid {
                    println!("  PID:      {pid}");
                }
            } else {
                return Err(resp.message);
            }
        }

        VmCommand::List => {
            let vms: Vec<VmListEntry> = client.get("/list-vms").await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&vms).unwrap());
            } else if vms.is_empty() {
                println!("No VMs running.");
            } else {
                println!("{:<38} {:<20} {:<16} {:<6} {:<8}", "ID", "NAME", "SSH HOST", "PORT", "PID");
                println!("{}", "-".repeat(90));
                for vm in &vms {
                    println!(
                        "{:<38} {:<20} {:<16} {:<6} {:<8}",
                        vm.id, vm.name, vm.ssh_host, vm.ssh_port, vm.pid
                    );
                }
            }
        }

        VmCommand::Delete { id } => {
            let msg = client.delete("/delete-vm", &DeleteVmRequest { id }).await?;
            if json {
                println!("{}", serde_json::json!({ "message": msg }));
            } else {
                println!("{msg}");
            }
        }
    }

    Ok(())
}
