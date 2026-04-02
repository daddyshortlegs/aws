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
    /// SSH host to connect to: "localhost" in user mode, "10.0.0.x" in bridge mode,
    /// or empty string if the VM hasn't received a DHCP lease yet.
    pub ssh_host: String,
    pub ssh_port: u16,
    pub pid: u32,
}

fn parse_arp_output(output: &str, mac: &str) -> Option<String> {
    // Parses "ip neigh show dev br0" output.
    // Line format: "10.0.0.15 dev br0 lladdr 52:54:00:ab:cd:ef REACHABLE"
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 && parts[4].eq_ignore_ascii_case(mac) {
            return Some(parts[0].to_string());
        }
    }
    None
}

fn parse_lease_output(content: &str, mac: &str) -> Option<String> {
    // Parses /var/lib/misc/dnsmasq.leases.
    // Line format: "<expiry> <mac> <ip> <hostname> <client-id>"
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[1].eq_ignore_ascii_case(mac) {
            return Some(parts[2].to_string());
        }
    }
    None
}

async fn lookup_ip_by_mac(mac: &str) -> Option<String> {
    // Prefer the dnsmasq lease file — it's populated immediately on DHCP grant,
    // whereas ARP entries only appear after the host exchanges IP traffic with the VM.
    const LEASE_FILE: &str = "/var/lib/misc/dnsmasq.leases";
    if let Ok(content) = tokio::fs::read_to_string(LEASE_FILE).await {
        if let Some(ip) = parse_lease_output(&content, mac) {
            return Some(ip);
        }
    }

    // Fall back to ARP table in case the lease file isn't available.
    let output = tokio::process::Command::new("ip")
        .args(["neigh", "show", "dev", "br0"])
        .output()
        .await
        .ok()?;
    parse_arp_output(&String::from_utf8_lossy(&output.stdout), mac)
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
                    },
                    NetworkMode::Bridge => {
                        let mac = vm.mac_address.as_deref().unwrap_or("");
                        let ip = lookup_ip_by_mac(mac).await.unwrap_or_default();
                        VmListEntry {
                            id: vm.id,
                            name: vm.name,
                            ssh_host: ip,
                            ssh_port: 22,
                            pid: vm.pid,
                        }
                    }
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

    #[test]
    fn test_parse_lease_output() {
        let content = "1234567890 52:54:00:ab:cd:ef 10.0.0.188 alpine-vm *\n\
                       1234567891 52:54:00:11:22:33 10.0.0.189 alpine-vm2 *\n";

        assert_eq!(
            parse_lease_output(content, "52:54:00:ab:cd:ef"),
            Some("10.0.0.188".to_string())
        );
        // Case-insensitive
        assert_eq!(
            parse_lease_output(content, "52:54:00:AB:CD:EF"),
            Some("10.0.0.188".to_string())
        );
        assert_eq!(parse_lease_output(content, "52:54:00:ff:ff:ff"), None);
    }

    #[test]
    fn test_parse_arp_output() {
        let output = "10.0.0.15 dev br0 lladdr 52:54:00:ab:cd:ef REACHABLE\n\
                      10.0.0.16 dev br0 lladdr 52:54:00:11:22:33 STALE\n";

        assert_eq!(
            parse_arp_output(output, "52:54:00:ab:cd:ef"),
            Some("10.0.0.15".to_string())
        );
        assert_eq!(
            parse_arp_output(output, "52:54:00:11:22:33"),
            Some("10.0.0.16".to_string())
        );
        // Case-insensitive
        assert_eq!(
            parse_arp_output(output, "52:54:00:AB:CD:EF"),
            Some("10.0.0.15".to_string())
        );
        assert_eq!(parse_arp_output(output, "52:54:00:ff:ff:ff"), None);
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
