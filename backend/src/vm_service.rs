use crate::config::Config;
use crate::qemu::vm_start;
use crate::vm_db::{delete_vm_by_id, get_vm_by_id, list_vms, store_vm_info, VmInfo};
use axum::{http::StatusCode, response::IntoResponse, Json};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, error, info, warn};
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
        error!("Failed to copy QCOW2 file from {source_qcow2:?} to {target_qcow2:?}: {e}");
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

            let _ = store_vm_info(&config.storage.metadata_dir, &vm_info);

            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            error!("Failed to launch VM {}: {e}", payload.name);
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
    list_vms_response(&Config::get_vms_dir())
}

fn list_vms_response(dir: &std::path::Path) -> axum::response::Response {
    match list_vms(dir) {
        Ok(vms) => (StatusCode::OK, axum::Json(vms)).into_response(),
        Err(e) => {
            error!("Failed to list VMs: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn start_all_vms() {
    let vms_dir = Config::get_vms_dir();
    let vms = list_vms(&vms_dir).unwrap();

    for vm in vms {
        let vm_info = get_vm_by_id(&vms_dir, &vm.id).unwrap().unwrap();
        let uuid = vm_info.id;
        let vm_name = vm_info.name;
        let ssh_port = vm_info.ssh_port;
        let qcow2_file = vms_dir.join(format!("{vm_name}.qcow2"));
        let output = vm_start(qcow2_file.to_str().unwrap(), ssh_port);
        match output {
            Ok(child) => {
                let vm_info = VmInfo {
                    id: uuid,
                    name: vm_name,
                    ssh_port,
                    pid: child.id().unwrap(),
                };

                let _ = store_vm_info(&vms_dir, &vm_info);

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

    let config = Config::load().expect("Failed to load configuration");

    delete_vm_response(
        &config.storage.metadata_dir,
        &config.storage.qcow2_dir,
        &payload.id,
    )
    .await
}

async fn delete_vm_response(
    metadata_dir: &std::path::Path,
    qcow2_dir: &std::path::Path,
    id: &str,
) -> axum::response::Response {
    match get_vm_by_id(metadata_dir, id) {
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

                    // Delete the corresponding QCOW2 file
                    let qcow2_file_path = qcow2_dir.join(format!("{}.qcow2", vm_info.name));

                    if let Err(e) = fs::remove_file(&qcow2_file_path).await {
                        warn!("Could not delete QCOW2 file: {qcow2_file_path:?} - {e}");
                        // Don't fail the entire operation if QCOW2 deletion fails
                        // The JSON metadata is more important to clean up
                    } else {
                        info!("Successfully deleted QCOW2 file: {qcow2_file_path:?}");
                    }

                    let _ = delete_vm_by_id(metadata_dir, &vm_info.id);

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tempfile::TempDir;

    // ── list_vms_response ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_vms_response_empty_dir() {
        let dir = TempDir::new().unwrap();
        let resp = list_vms_response(dir.path());
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let vms: Vec<VmInfo> = serde_json::from_slice(&body).unwrap();
        assert!(vms.is_empty());
    }

    #[tokio::test]
    async fn test_list_vms_response_returns_stored_vms() {
        let dir = TempDir::new().unwrap();
        let vm = VmInfo {
            id: "abc-1".to_string(),
            name: "my-vm".to_string(),
            ssh_port: 55000,
            pid: 42,
        };
        store_vm_info(dir.path(), &vm).unwrap();

        let resp = list_vms_response(dir.path());
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let vms: Vec<VmInfo> = serde_json::from_slice(&body).unwrap();
        assert_eq!(vms.len(), 1);
        assert_eq!(vms[0].id, "abc-1");
        assert_eq!(vms[0].ssh_port, 55000);
    }

    // ── delete_vm_response ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_vm_response_not_found() {
        let meta_dir = TempDir::new().unwrap();
        let qcow2_dir = TempDir::new().unwrap();

        let resp = delete_vm_response(meta_dir.path(), qcow2_dir.path(), "no-such-id").await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_vm_response_corrupted_metadata_returns_500() {
        let meta_dir = TempDir::new().unwrap();
        let qcow2_dir = TempDir::new().unwrap();

        // Write a .json file that is not valid VmInfo JSON
        std::fs::write(meta_dir.path().join("bad-id.json"), "not json").unwrap();

        let resp = delete_vm_response(meta_dir.path(), qcow2_dir.path(), "bad-id").await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
