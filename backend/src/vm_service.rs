use crate::config::{Config, NetworkMode};
use crate::qemu::{
    is_process_running, mac_from_uuid, send_monitor_command, vm_start, NetworkConfig,
};
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
    /// Whether the QEMU process is currently alive.
    pub running: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StopVmRequest {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartVmRequest {
    pub id: String,
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

    let monitor_socket = config.storage.metadata_dir.join(format!("{uuid}.monitor"));

    match vm_start(
        target_qcow2.to_str().unwrap(),
        &network,
        monitor_socket.to_str().unwrap(),
    ) {
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
                        running: is_process_running(vm.pid),
                        id: vm.id,
                        name: vm.name,
                        ssh_host: "localhost".to_string(),
                        ssh_port: vm.ssh_port.unwrap_or(0),
                        pid: vm.pid,
                        mac_address: None,
                    },
                    NetworkMode::Bridge => VmListEntry {
                        running: is_process_running(vm.pid),
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

/// Launch a single VM from its persisted metadata. Updates the stored PID on
/// success. Called both by `start_all_vms` on startup and `start_vm_handler`
/// on demand.
async fn start_single_vm(
    vm_info: &VmInfo,
    metadata_dir: &Path,
    qcow2_dir: &Path,
    network_mode: &NetworkMode,
) -> Result<u32, String> {
    let qcow2_file = qcow2_dir.join(format!("{}.qcow2", vm_info.name));
    let monitor_socket = metadata_dir.join(format!("{}.monitor", vm_info.id));

    // Remove any stale monitor socket from a previous run.
    let _ = fs::remove_file(&monitor_socket).await;

    let (network, ssh_port, mac_address) = match network_mode {
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

    match vm_start(
        qcow2_file.to_str().unwrap(),
        &network,
        monitor_socket.to_str().unwrap(),
    ) {
        Ok(child) => {
            let pid = child.id().unwrap();
            let updated = VmInfo {
                id: vm_info.id.clone(),
                name: vm_info.name.clone(),
                ssh_port,
                mac_address,
                pid,
            };
            let _ = store_vm_info(metadata_dir, &updated);
            Ok(pid)
        }
        Err(e) => Err(e.to_string()),
    }
}

pub async fn start_all_vms() {
    let config = Config::load().expect("Failed to load configuration");
    let vms = list_vms(&config.storage.metadata_dir).unwrap_or_default();

    for vm in vms {
        let name = vm.name.clone();
        match start_single_vm(
            &vm,
            &config.storage.metadata_dir,
            &config.storage.qcow2_dir,
            &config.network_mode,
        )
        .await
        {
            Ok(pid) => info!("VM {name} started with PID {pid}"),
            Err(e) => error!("Failed to start VM {name}: {e}"),
        }
    }
}

pub async fn stop_vm_handler(Json(payload): Json<StopVmRequest>) -> impl IntoResponse {
    info!("Stopping VM: {}", payload.id);
    let config = Config::load().expect("Failed to load configuration");
    stop_vm_response(&config.storage.metadata_dir, &payload.id).await
}

async fn stop_vm_response(metadata_dir: &Path, id: &str) -> axum::response::Response {
    match get_vm_by_id(metadata_dir, id) {
        Ok(Some(vm_info)) => {
            if !is_process_running(vm_info.pid) {
                return (StatusCode::CONFLICT, "VM is not running").into_response();
            }
            let socket_path = metadata_dir.join(format!("{}.monitor", vm_info.id));
            match send_monitor_command(socket_path.to_str().unwrap(), "system_powerdown").await {
                Ok(_) => {
                    info!("Sent system_powerdown to VM {}", vm_info.name);
                    (StatusCode::OK, "VM shutdown initiated").into_response()
                }
                Err(e) => {
                    error!("Failed to reach monitor for VM {}: {e}", vm_info.name);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to stop VM: {e}"),
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

pub async fn start_vm_handler(Json(payload): Json<StartVmRequest>) -> impl IntoResponse {
    info!("Starting VM: {}", payload.id);
    let config = Config::load().expect("Failed to load configuration");
    start_vm_response(
        &config.storage.metadata_dir,
        &config.storage.qcow2_dir,
        &config.network_mode,
        &payload.id,
    )
    .await
}

async fn start_vm_response(
    metadata_dir: &Path,
    qcow2_dir: &Path,
    network_mode: &NetworkMode,
    id: &str,
) -> axum::response::Response {
    match get_vm_by_id(metadata_dir, id) {
        Ok(Some(vm_info)) => {
            if is_process_running(vm_info.pid) {
                return (StatusCode::CONFLICT, "VM is already running").into_response();
            }
            match start_single_vm(&vm_info, metadata_dir, qcow2_dir, network_mode).await {
                Ok(pid) => {
                    info!("VM {} restarted with PID {pid}", vm_info.name);
                    (StatusCode::OK, format!("VM started with PID {pid}")).into_response()
                }
                Err(e) => {
                    error!("Failed to start VM {}: {e}", vm_info.name);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to start VM: {e}"),
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

    #[tokio::test]
    async fn test_list_vms_response_running_true_when_process_alive() {
        let dir = TempDir::new().unwrap();
        let vm = VmInfo {
            id: "abc-1".to_string(),
            name: "my-vm".to_string(),
            ssh_port: Some(55000),
            mac_address: None,
            pid: std::process::id(), // current test process is definitely alive
        };
        store_vm_info(dir.path(), &vm).unwrap();

        let resp = list_vms_response(dir.path(), &NetworkMode::User).await;
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let vms: Vec<VmListEntry> = serde_json::from_slice(&body).unwrap();
        assert!(vms[0].running);
    }

    #[tokio::test]
    async fn test_list_vms_response_running_false_when_process_dead() {
        let dir = TempDir::new().unwrap();
        let vm = VmInfo {
            id: "abc-1".to_string(),
            name: "my-vm".to_string(),
            ssh_port: Some(55000),
            mac_address: None,
            pid: u32::MAX, // guaranteed not to be a running process
        };
        store_vm_info(dir.path(), &vm).unwrap();

        let resp = list_vms_response(dir.path(), &NetworkMode::User).await;
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let vms: Vec<VmListEntry> = serde_json::from_slice(&body).unwrap();
        assert!(!vms[0].running);
    }

    // ── stop_vm_response ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_stop_vm_response_not_found() {
        let meta_dir = TempDir::new().unwrap();
        let resp = stop_vm_response(meta_dir.path(), "no-such-id").await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_stop_vm_response_not_running_returns_conflict() {
        let meta_dir = TempDir::new().unwrap();
        let vm = VmInfo {
            id: "vm-1".to_string(),
            name: "test".to_string(),
            ssh_port: Some(22222),
            mac_address: None,
            pid: u32::MAX, // not running
        };
        store_vm_info(meta_dir.path(), &vm).unwrap();

        let resp = stop_vm_response(meta_dir.path(), "vm-1").await;
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_stop_vm_response_sends_powerdown_to_monitor_socket() {
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixListener;
        use tokio::sync::Mutex;

        let meta_dir = TempDir::new().unwrap();
        let vm = VmInfo {
            id: "vm-1".to_string(),
            name: "test".to_string(),
            ssh_port: Some(22222),
            mac_address: None,
            pid: std::process::id(), // current process is alive
        };
        store_vm_info(meta_dir.path(), &vm).unwrap();

        // Set up a mock monitor socket
        let socket_path = meta_dir.path().join("vm-1.monitor");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let received = Arc::new(Mutex::new(String::new()));
        let received_clone = received.clone();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            stream.write_all(b"(qemu) ").await.unwrap();
            let mut buf = vec![0u8; 64];
            if let Ok(n) = stream.read(&mut buf).await {
                *received_clone.lock().await = String::from_utf8_lossy(&buf[..n]).to_string();
            }
        });

        tokio::task::yield_now().await;

        let resp = stop_vm_response(meta_dir.path(), "vm-1").await;
        assert_eq!(resp.status(), StatusCode::OK);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(received.lock().await.contains("system_powerdown"));
    }

    // ── start_vm_response ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_start_vm_response_not_found() {
        let meta_dir = TempDir::new().unwrap();
        let qcow2_dir = TempDir::new().unwrap();
        let resp = start_vm_response(
            meta_dir.path(),
            qcow2_dir.path(),
            &NetworkMode::User,
            "no-such-id",
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_start_vm_response_already_running_returns_conflict() {
        let meta_dir = TempDir::new().unwrap();
        let qcow2_dir = TempDir::new().unwrap();
        let vm = VmInfo {
            id: "vm-1".to_string(),
            name: "test".to_string(),
            ssh_port: Some(22222),
            mac_address: None,
            pid: std::process::id(), // current process is alive
        };
        store_vm_info(meta_dir.path(), &vm).unwrap();

        let resp = start_vm_response(
            meta_dir.path(),
            qcow2_dir.path(),
            &NetworkMode::User,
            "vm-1",
        )
        .await;
        assert_eq!(resp.status(), StatusCode::CONFLICT);
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
