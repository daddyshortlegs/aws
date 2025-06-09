use axum::{
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use std::path::PathBuf;
use tokio::fs;
use rand::Rng;

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
}

pub async fn launch_vm(
    Json(payload): Json<LaunchVmRequest>,
) -> (StatusCode, Json<LaunchVmResponse>) {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let source_qcow2 = current_dir.join("ubuntu.qcow2");
    let target_qcow2 = current_dir.join(format!("{}.qcow2", payload.name));

    // Generate random port in ephemeral range (49152-65535)
    let ssh_port = rand::thread_rng().gen_range(49152..65535);

    // Copy the QCOW2 file
    if let Err(e) = fs::copy(&source_qcow2, &target_qcow2).await {
        let response = LaunchVmResponse {
            success: false,
            message: format!("Failed to copy QCOW2 file: {}", e),
            instance_id: None,
            ssh_port: None,
        };
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
    }

    // Execute QEMU command
    let output = Command::new("qemu-system-x86_64")
        .args([
            "-m", "8192",
            "-smp", "6",
            "-drive", &format!("file={}", target_qcow2.to_str().unwrap()),
            "-boot", "d",
            "-vga", "virtio",
            "-netdev", &format!("user,id=net0,hostfwd=tcp::{}:-:22", ssh_port),
            "-device", "e1000,netdev=net0"
        ])
        .spawn();

    match output {
        Ok(_) => {
            let response = LaunchVmResponse {
                success: true,
                message: format!("VM launch request received for {} in {}", payload.name, payload.region),
                instance_id: Some("qemu-instance".to_string()),
                ssh_port: Some(ssh_port),
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let response = LaunchVmResponse {
                success: false,
                message: format!("Failed to launch VM: {}", e),
                instance_id: None,
                ssh_port: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
} 