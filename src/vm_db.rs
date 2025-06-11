use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct VmInfo {
    pub id: String,
    pub name: String,
    pub ssh_port: u16,
    pub pid: u32,
}

pub fn store_vm_info(vm_info: &VmInfo) -> std::io::Result<()> {
    println!("Storing VM info: {:?}", vm_info);

    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let vms_dir = current_dir.join("vms");

    // Create vms directory if it doesn't exist
    if !vms_dir.exists() {
        fs::create_dir(&vms_dir)?;
    }

    let file_path = vms_dir.join(format!("{}.json", vm_info.id));
    let json = serde_json::to_string_pretty(vm_info)?;
    println!("Writing VM info to: {:?}", file_path);
    println!("VM info: {:?}", json);
    fs::write(file_path, json)
}
