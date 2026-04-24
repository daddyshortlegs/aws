use crate::config::{Config, NetworkMode};
use crate::qemu::{mac_from_uuid, vm_start, NetworkConfig};
use crate::vm_db::{delete_vm_by_id, get_vm_by_id, list_vms, store_vm_info, VmInfo};
use axum::{http::StatusCode, response::IntoResponse, Json};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
    pub ssh_host: Option<String>,
    pub ssh_port: Option<u16>,
    pub pid: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VmListEntry {
    pub id: String,
    pub name: String,
    /// SSH host to connect to: "localhost" in user mode, empty in bridge mode
    /// (the proxy resolves the IP from the dnsmasq lease file).
    pub ssh_host: String,
    pub ssh_port: u16,
    pub pid: u32,
    /// MAC address included in bridge mode so the proxy can resolve the IP.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
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

    if let Err(e) = fs::copy(&source_qcow2, &target_qcow2).await {
        error!("Failed to copy QCOW2 file from {source_qcow2:?} to {target_qcow2:?}: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LaunchVmResponse {
                success: false,
                message: format!("Failed to copy QCOW2 file: {e}"),
                instance_id: None,
                ssh_host: None,
                ssh_port: None,
                pid: None,
            }),
        );
    }

    let uuid = Uuid::new_v4().to_string();

    let (network, vm_info_ssh_port, vm_info_mac, response_ssh_host, response_ssh_port) =
        match config.network_mode {
            NetworkMode::User => {
                let ssh_port: u16 = rand::thread_rng().gen_range(49152..65535);
                (
                    NetworkConfig::User { ssh_port },
                    Some(ssh_port),
                    None,
                    "localhost".to_string(),
                    ssh_port,
                )
            }
            NetworkMode::Bridge => {
                let mac = mac_from_uuid(&uuid);
                (
                    NetworkConfig::Bridge {
                        mac_address: mac.clone(),
                    },
                    None,
                    Some(mac),
                    // IP isn't known yet; list-vms will resolve it via ARP lookup
                    String::new(),
                    22,
                )
            }
        };

    match vm_start(target_qcow2.to_str().unwrap(), &network) {
        Ok(child) => {
            let vm_info = VmInfo {
                id: uuid.clone(),
                name: payload.name.clone(),
                ssh_port: vm_info_ssh_port,
                mac_address: vm_info_mac,
                pid: child.id().unwrap(),
            };
            let _ = store_vm_info(&config.storage.metadata_dir, &vm_info);

            (
                StatusCode::OK,
                Json(LaunchVmResponse {
                    success: true,
                    message: format!(
                        "VM launch request received for {} in {}",
                        payload.name, payload.region
                    ),
                    instance_id: Some(uuid),
                    ssh_host: Some(response_ssh_host),
                    ssh_port: Some(response_ssh_port),
                    pid: child.id(),
                }),
            )
        }
        Err(e) => {
            error!("Failed to launch VM {}: {e}", payload.name);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LaunchVmResponse {
                    success: false,
                    message: format!("Failed to launch VM: {e}"),
                    instance_id: None,
                    ssh_host: None,
                    ssh_port: None,
                    pid: None,
                }),
            )
        }
    }
}

pub async fn list_vms_handler() -> impl IntoResponse {
    let config = Config::load().expect("Failed to load configuration");
    list_vms_response(&config.storage.metadata_dir, &config.network_mode).await
}

async fn list_vms_response(dir: &Path, mode: &NetworkMode) -> axum::response::Response {
    match list_vms(dir) {
        Ok(vms) => {
            let mut entries = Vec::new();
            for vm in vms {
                let entry = match mode {
                    NetworkMode::User => VmListEntry {
                        id: vm.id,
                        name: vm.name,
                        ssh_host: "localhost".to_string(),
                        ssh_port: vm.ssh_port.unwrap_or(0),
                        pid: vm.pid,
                        mac_address: None,
                    },
                    NetworkMode::Bridge => VmListEntry {
                        id: vm.id,
                        name: vm.name,
                        // Leave ssh_host empty; the proxy resolves it from the
                        // dnsmasq lease file on the controller node.
                        ssh_host: String::new(),
                        ssh_port: 22,
                        pid: vm.pid,
                        mac_address: vm.mac_address.clone(),
                    },
                };
                entries.push(entry);
            }
            (StatusCode::OK, Json(entries)).into_response()
        }
        Err(e) => {
            error!("Failed to list VMs: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn start_all_vms() {
    let config = Config::load().expect("Failed to load configuration");
    let vms_dir = config.storage.metadata_dir.clone();
    let vms = list_vms(&vms_dir).unwrap();

    for vm in vms {
        let vm_info = get_vm_by_id(&vms_dir, &vm.id).unwrap().unwrap();
        let uuid = vm_info.id.clone();
        let vm_name = vm_info.name.clone();
        let qcow2_file = vms_dir.join(format!("{vm_name}.qcow2"));

        let (network, vm_info_ssh_port, vm_info_mac) = match config.network_mode {
            NetworkMode::User => {
                let port = vm_info
                    .ssh_port
                    .unwrap_or_else(|| rand::thread_rng().gen_range(49152..65535));
                (NetworkConfig::User { ssh_port: port }, Some(port), None)
            }
            NetworkMode::Bridge => {
                let mac = vm_info
                    .mac_address
                    .clone()
                    .unwrap_or_else(|| mac_from_uuid(&vm_info.id));
                (
                    NetworkConfig::Bridge {
                        mac_address: mac.clone(),
                    },
                    None,
                    Some(mac),
                )
            }
        };

        match vm_start(qcow2_file.to_str().unwrap(), &network) {
            Ok(child) => {
                let updated = VmInfo {
                    id: uuid,
                    name: vm_name.clone(),
                    ssh_port: vm_info_ssh_port,
                    mac_address: vm_info_mac,
                    pid: child.id().unwrap(),
                };
                let _ = store_vm_info(&vms_dir, &updated);
                info!("VM {} started with PID: {}", vm_name, child.id().unwrap());
            }
            Err(e) => {
                error!("Failed to start VM {}: {}", vm_name, e);
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
    metadata_dir: &Path,
    qcow2_dir: &Path,
    id: &str,
) -> axum::response::Response {
    match get_vm_by_id(metadata_dir, id) {
        Ok(Some(vm_info)) => match kill(Pid::from_raw(vm_info.pid as i32), Signal::SIGTERM) {
            Ok(_) => {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                if kill(Pid::from_raw(vm_info.pid as i32), Signal::SIGKILL).is_ok() {
                    warn!("Process {} was still running, force killed", vm_info.pid);
                }

                let qcow2_file_path = qcow2_dir.join(format!("{}.qcow2", vm_info.name));

                if let Err(e) = fs::remove_file(&qcow2_file_path).await {
                    warn!("Could not delete QCOW2 file: {qcow2_file_path:?} - {e}");
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
        },
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
        let resp = list_vms_response(dir.path(), &NetworkMode::User).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let vms: Vec<VmListEntry> = serde_json::from_slice(&body).unwrap();
        assert!(vms.is_empty());
    }

    #[tokio::test]
    async fn test_list_vms_response_returns_stored_vms() {
        let dir = TempDir::new().unwrap();
        let vm = VmInfo {
            id: "abc-1".to_string(),
            name: "my-vm".to_string(),
            ssh_port: Some(55000),
            mac_address: None,
            pid: 42,
        };
        store_vm_info(dir.path(), &vm).unwrap();

        let resp = list_vms_response(dir.path(), &NetworkMode::User).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let vms: Vec<VmListEntry> = serde_json::from_slice(&body).unwrap();
        assert_eq!(vms.len(), 1);
        assert_eq!(vms[0].id, "abc-1");
        assert_eq!(vms[0].ssh_host, "localhost");
        assert_eq!(vms[0].ssh_port, 55000);
    }

    #[tokio::test]
    async fn test_list_vms_response_bridge_mode_includes_mac_address() {
        let dir = TempDir::new().unwrap();
        let vm = VmInfo {
            id: "abc-1".to_string(),
            name: "my-vm".to_string(),
            ssh_port: None,
            mac_address: Some("52:54:00:ab:cd:ef".to_string()),
            pid: 42,
        };
        store_vm_info(dir.path(), &vm).unwrap();

        let resp = list_vms_response(dir.path(), &NetworkMode::Bridge).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let vms: Vec<VmListEntry> = serde_json::from_slice(&body).unwrap();
        assert_eq!(vms.len(), 1);
        // MAC is passed through for the proxy to resolve.
        assert_eq!(vms[0].mac_address.as_deref(), Some("52:54:00:ab:cd:ef"));
        // ssh_host is left empty; the proxy fills it in.
        assert_eq!(vms[0].ssh_host, "");
        assert_eq!(vms[0].ssh_port, 22);
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

        std::fs::write(meta_dir.path().join("bad-id.json"), "not json").unwrap();

        let resp = delete_vm_response(meta_dir.path(), qcow2_dir.path(), "bad-id").await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
