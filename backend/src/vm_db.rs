use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct VmInfo {
    pub id: String,
    pub name: String,
    pub ssh_port: u16,
    pub pid: u32,
}

pub fn store_vm_info(vm_info: &VmInfo) -> std::io::Result<()> {
    debug!("Storing VM info: {vm_info:?}");

    let file_path = create_file_path(&vm_info.id)?;

    let json = serde_json::to_string_pretty(&vm_info)?;
    debug!("Writing VM info to: {file_path:?}");
    debug!("VM info: {json:?}");
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
    let file_path = vms_dir.join(format!("{id}.json"));
    Ok(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;
    use tempfile::TempDir;

    static INIT: Once = Once::new();
    static mut TEST_DIR: Option<TempDir> = None;

    fn setup_test_env() -> &'static TempDir {
        unsafe {
            INIT.call_once(|| {
                TEST_DIR = Some(TempDir::new().unwrap());
            });
            TEST_DIR.as_ref().unwrap()
        }
    }

    fn create_test_vm(id: &str, name: &str) -> VmInfo {
        VmInfo {
            id: id.to_string(),
            name: name.to_string(),
            ssh_port: 2222,
            pid: 1234,
        }
    }

    #[test]
    fn test_store_and_get_vm() {
        let test_dir = setup_test_env();
        let vm = create_test_vm("test-1", "Test VM 1");

        // Store the VM info
        store_vm_info(&vm).unwrap();

        // Get the VM info
        let retrieved = get_vm_by_id("test-1").unwrap().unwrap();
        assert_eq!(retrieved.id, vm.id);
        assert_eq!(retrieved.name, vm.name);
        assert_eq!(retrieved.ssh_port, vm.ssh_port);
        assert_eq!(retrieved.pid, vm.pid);
    }

    #[test]
    fn test_get_nonexistent_vm() {
        let test_dir = setup_test_env();
        let result = get_vm_by_id("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_vms() {
        let test_dir = setup_test_env();
        let vm1 = create_test_vm("test-2", "Test VM 2");
        let vm2 = create_test_vm("test-3", "Test VM 3");

        // Store multiple VMs
        store_vm_info(&vm1).unwrap();
        store_vm_info(&vm2).unwrap();

        // List VMs
        let vms = list_vms().unwrap();

        // Verify the VMs are in the list
        let ids: Vec<String> = vms.iter().map(|v| v.id.clone()).collect();
        assert!(ids.contains(&vm1.id));
        assert!(ids.contains(&vm2.id));
    }

    #[test]
    fn test_delete_vm() {
        let test_dir = setup_test_env();
        let vm = create_test_vm("test-4", "Test VM 4");

        // Store the VM
        store_vm_info(&vm).unwrap();

        // Delete the VM
        let result = delete_vm_by_id("test-4").unwrap();
        assert!(result.is_none());

        // Verify it's gone
        let result = get_vm_by_id("test-4").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_nonexistent_vm() {
        let test_dir = setup_test_env();
        let result = delete_vm_by_id("nonexistent").unwrap();
        assert!(result.is_none());
    }
}
