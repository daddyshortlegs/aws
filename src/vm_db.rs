use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct VmInfo {
    pub id: String,
    pub name: String,
    pub ssh_port: u16,
    pub pid: u32,
}

pub fn store_vm_info(vm_info: &VmInfo) -> std::io::Result<()> {
    println!("Storing VM info: {:?}", vm_info);

    let file_path = create_file_path(&vm_info.id)?;

    let json = serde_json::to_string_pretty(vm_info)?;
    println!("Writing VM info to: {:?}", file_path);
    println!("VM info: {:?}", json);
    fs::write(file_path, json)
}

pub fn list_vms() -> std::io::Result<Vec<VmInfo>> {
    let vms_dir = Config::get_vms_dir();

    if !vms_dir.exists() {
        return Ok(Vec::new());
    }

    let mut vms = Vec::new();
    for entry in fs::read_dir(vms_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(vm_info) = serde_json::from_str(&contents) {
                    vms.push(vm_info);
                }
            }
        }
    }
    Ok(vms)
}

pub fn get_vm_by_id(id: &str) -> std::io::Result<Option<VmInfo>> {
    let file_path = create_file_path(id)?;

    if !file_path.exists() {
        return Ok(None);
    }

    match fs::read_to_string(file_path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(vm_info) => Ok(Some(vm_info)),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        },
        Err(e) => Err(e),
    }
}

pub fn delete_vm_by_id(id: &str) -> std::io::Result<Option<VmInfo>> {
    let file_path = create_file_path(id)?;

    if !file_path.exists() {
        return Ok(None);
    }

    fs::remove_file(file_path)?;
    Ok(None)
}

fn create_file_path(id: &str) -> std::io::Result<PathBuf> {
    let vms_dir = Config::get_vms_dir();
    let file_path = vms_dir.join(format!("{}.json", id));
    Ok(file_path)
}
