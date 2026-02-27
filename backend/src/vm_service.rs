use crate::config::Config;
use crate::qemu::vm_start;
use crate::vm_db::{delete_vm_by_id, get_vm_by_id, list_vms, store_vm_info, VmInfo};
use axum::{http::StatusCode, response::IntoResponse, Json};
use tracing::{debug, error, info, warn};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

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
    let source_qcow2 = current_dir.join("alpine.qcow2");

    let config = Config::load().expect("Failed to load configuration");
    let target_qcow2 = config
        .storage
        .qcow2_dir
        .join(format!("{}.qcow2", payload.name));

    debug!("source_qcow2: {source_qcow2:?}");
    debug!("target_qcow2: {target_qcow2:?}");

    // Copy the QCOW2 file
    if let Err(e) = fs::copy(&source_qcow2, &target_qcow2).await {
        let response = LaunchVmResponse {
            success: false,
            message: format!("Failed to copy QCOW2 file: {e}"),
            instance_id: None,
            ssh_port: None,
            pid: None,
        };
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
    }

    // Generate random port in ephemeral range (49152-65535)
    let ssh_port = rand::thread_rng().gen_range(49152..65535);

    let output = vm_start(target_qcow2.to_str().unwrap(), ssh_port);

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
                ssh_port,
                pid: child.id().unwrap(),
            };

            let _ = store_vm_info(&vm_info);

            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let response = LaunchVmResponse {
                success: false,
                message: format!("Failed to launch VM: {e}"),
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

pub async fn start_all_vms() {
    let vms = list_vms().unwrap();

    for vm in vms {
        let vm_info = get_vm_by_id(&vm.id).unwrap().unwrap();
        let uuid = vm_info.id;
        let vm_name = vm_info.name;
        let ssh_port = vm_info.ssh_port;
        let qcow2_file = Config::get_vms_dir().join(format!("{vm_name}.qcow2"));
        let output = vm_start(qcow2_file.to_str().unwrap(), ssh_port);
        match output {
            Ok(child) => {
                let vm_info = VmInfo {
                    id: uuid,
                    name: vm_name,
                    ssh_port,
                    pid: child.id().unwrap(),
                };

                let _ = store_vm_info(&vm_info);

                info!("VM {} started with PID: {}", vm.name, child.id().unwrap());
            }
            Err(e) => {
                error!("Failed to start VM {}: {}", vm.name, e);
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteVmRequest {
    pub id: String,
}

pub async fn delete_vm_handler(Json(payload): Json<DeleteVmRequest>) -> impl IntoResponse {
    info!("Deleting VM: {payload:?}");

    match get_vm_by_id(&payload.id) {
        Ok(Some(vm_info)) => {
            // Try to terminate the process
            match kill(Pid::from_raw(vm_info.pid as i32), Signal::SIGTERM) {
                Ok(_) => {
                    // Wait a bit for the process to terminate gracefully
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    // Check if process is still running, if so, force kill it
                    if kill(Pid::from_raw(vm_info.pid as i32), Signal::SIGKILL).is_ok() {
                        warn!("Process {} was still running, force killed", vm_info.pid);
                    }

                    // Delete the JSON file using the same path as other operations
                    let config = Config::load().expect("Failed to load configuration");
                    let file_path = config
                        .storage
                        .metadata_dir
                        .join(format!("{}.json", vm_info.id));

                    if let Err(e) = fs::remove_file(&file_path).await {
                        error!("Error deleting VM info file: {file_path:?} - {e}");
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to delete VM info file",
                        )
                            .into_response();
                    }

                    // Delete the corresponding QCOW2 file
                    let qcow2_file_path = config
                        .storage
                        .qcow2_dir
                        .join(format!("{}.qcow2", vm_info.name));

                    if let Err(e) = fs::remove_file(&qcow2_file_path).await {
                        warn!("Could not delete QCOW2 file: {qcow2_file_path:?} - {e}");
                        // Don't fail the entire operation if QCOW2 deletion fails
                        // The JSON metadata is more important to clean up
                    } else {
                        info!("Successfully deleted QCOW2 file: {qcow2_file_path:?}");
                    }

                    let _ = delete_vm_by_id(&vm_info.id);

                    (StatusCode::OK, "VM successfully terminated and removed").into_response()
                }
                Err(e) => {
                    error!("Error terminating process {}: {}", vm_info.pid, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to terminate VM: {e}"),
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, "VM not found").into_response(),
        Err(e) => {
            error!("Error retrieving VM info: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error retrieving VM info: {e}"),
            )
                .into_response()
        }
    }
}
