use crate::vm_db::{delete_vm_by_id, get_vm_by_id, list_vms, store_vm_info, VmInfo};
use axum::{http::StatusCode, response::IntoResponse, Json};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use tokio::fs;
use tokio::process::Command;
use uuid::Uuid;
use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchVmRequest {
    pub name: String,
    pub instance_type: String,
    pub region: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchVmResponse {
    pub success: bool,
    pub message: String,
    pub instance_id: Option<String>,
    pub ssh_port: Option<u16>,
    pub pid: Option<u32>,
}

pub async fn launch_vm(
    Json(payload): Json<LaunchVmRequest>,
) -> (StatusCode, Json<LaunchVmResponse>) {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let source_qcow2 = current_dir.join("ubuntu.qcow2");

    let config = Config::load().expect("Failed to load configuration");
    let target_qcow2 = config.storage.qcow2_dir.join(format!("{}.qcow2", payload.name));


    println!("target_qcow2: {:?}", target_qcow2);

    // Copy the QCOW2 file
    if let Err(e) = fs::copy(&source_qcow2, &target_qcow2).await {
        let response = LaunchVmResponse {
            success: false,
            message: format!("Failed to copy QCOW2 file: {}", e),
            instance_id: None,
            ssh_port: None,
            pid: None,
        };
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
    }

    // Generate random port in ephemeral range (49152-65535)
    let ssh_port = rand::thread_rng().gen_range(49152..65535);

    // Execute QEMU command
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args([
        "-m",
        "8192",
        "-smp",
        "6",
        "-drive",
        &format!("file={}", target_qcow2.to_str().unwrap()),
        "-boot",
        "d",
        "-vga",
        "virtio",
        "-netdev",
        &format!("user,id=net0,hostfwd=tcp::{}:-:22", ssh_port),
        "-device",
        "e1000,netdev=net0",
    ]);

    let output = cmd.spawn();

    match output {
        Ok(child) => {
            // generate random uuid
            let uuid = Uuid::new_v4();



            let response = LaunchVmResponse {
                success: true,
                message: format!(
                    "VM launch request received for {} in {}",
                    payload.name, payload.region
                ),
                instance_id: Some(uuid.to_string()),
                ssh_port: Some(ssh_port),
                pid: child.id(),
            };

            let vm_info = VmInfo {
                id: uuid.to_string(),
                name: payload.name,
                ssh_port: ssh_port,
                pid: child.id().unwrap(),
            };

            store_vm_info(&vm_info);

            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let response = LaunchVmResponse {
                success: false,
                message: format!("Failed to launch VM: {}", e),
                instance_id: None,
                ssh_port: None,
                pid: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

pub async fn list_vms_handler() -> impl IntoResponse {
    match list_vms() {
        Ok(vms) => (StatusCode::OK, axum::Json(vms)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteVmRequest {
    pub id: String,
}

pub async fn delete_vm_handler(Json(payload): Json<DeleteVmRequest>) -> impl IntoResponse {
    println!("Deleting VM: {:?}", payload);

    match get_vm_by_id(&payload.id) {
        Ok(Some(vm_info)) => {
            // Try to terminate the process
            match kill(Pid::from_raw(vm_info.pid as i32), Signal::SIGTERM) {
                Ok(_) => {
                    // Wait a bit for the process to terminate gracefully
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    // Check if process is still running, if so, force kill it
                    if let Ok(_) = kill(Pid::from_raw(vm_info.pid as i32), Signal::SIGKILL) {
                        println!("Process {} was still running, force killed", vm_info.pid);
                    }

                    // Delete the JSON file using the same path as other operations
                    let config = Config::load().expect("Failed to load configuration");
                    let file_path = config.storage.metadata_dir.join(format!("{}.json", vm_info.id));

                    if let Err(e) = fs::remove_file(&file_path).await {
                        println!("Error deleting VM info file: {:?} - {}", file_path, e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to delete VM info file",
                        )
                            .into_response();
                    }

                    delete_vm_by_id(&vm_info.id);

                    (StatusCode::OK, "VM successfully terminated and removed").into_response()
                }
                Err(e) => {
                    println!("Error terminating process {}: {}", vm_info.pid, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to terminate VM: {}", e),
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, "VM not found").into_response(),
        Err(e) => {
            println!("Error retrieving VM info: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error retrieving VM info: {}", e),
            )
                .into_response()
        }
    }
}
