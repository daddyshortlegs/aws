use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct VmInfo {
    pub id: String,
    pub name: String,
    pub ssh_port: u16,
    pub pid: u32,
}

pub fn store_vm_info(dir: &Path, vm_info: &VmInfo) -> std::io::Result<()> {
    debug!("Storing VM info: {vm_info:?}");

    let file_path = create_file_path(dir, &vm_info.id);

    let json = serde_json::to_string_pretty(&vm_info)?;
    debug!("Writing VM info to: {file_path:?}");
    debug!("VM info: {json:?}");
    fs::write(file_path, json)
}

pub fn list_vms(dir: &Path) -> std::io::Result<Vec<VmInfo>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut vms = Vec::new();
    for entry in fs::read_dir(dir)? {
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

pub fn get_vm_by_id(dir: &Path, id: &str) -> std::io::Result<Option<VmInfo>> {
    let file_path = create_file_path(dir, id);

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

pub fn delete_vm_by_id(dir: &Path, id: &str) -> std::io::Result<Option<VmInfo>> {
    let file_path = create_file_path(dir, id);

    if !file_path.exists() {
        return Ok(None);
    }

    fs::remove_file(file_path)?;
    Ok(None)
}

fn create_file_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
        let dir = TempDir::new().unwrap();
        let vm = create_test_vm("test-1", "Test VM 1");

        store_vm_info(dir.path(), &vm).unwrap();

        let retrieved = get_vm_by_id(dir.path(), "test-1").unwrap().unwrap();
        assert_eq!(retrieved.id, vm.id);
        assert_eq!(retrieved.name, vm.name);
        assert_eq!(retrieved.ssh_port, vm.ssh_port);
        assert_eq!(retrieved.pid, vm.pid);
    }

    #[test]
    fn test_get_nonexistent_vm() {
        let dir = TempDir::new().unwrap();
        let result = get_vm_by_id(dir.path(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_vms() {
        let dir = TempDir::new().unwrap();
        let vm1 = create_test_vm("test-2", "Test VM 2");
        let vm2 = create_test_vm("test-3", "Test VM 3");

        store_vm_info(dir.path(), &vm1).unwrap();
        store_vm_info(dir.path(), &vm2).unwrap();

        let vms = list_vms(dir.path()).unwrap();

        let ids: Vec<String> = vms.iter().map(|v| v.id.clone()).collect();
        assert!(ids.contains(&vm1.id));
        assert!(ids.contains(&vm2.id));
    }

    #[test]
    fn test_delete_vm() {
        let dir = TempDir::new().unwrap();
        let vm = create_test_vm("test-4", "Test VM 4");

        store_vm_info(dir.path(), &vm).unwrap();

        let result = delete_vm_by_id(dir.path(), "test-4").unwrap();
        assert!(result.is_none());

        let result = get_vm_by_id(dir.path(), "test-4").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_nonexistent_vm() {
        let dir = TempDir::new().unwrap();
        let result = delete_vm_by_id(dir.path(), "nonexistent").unwrap();
        assert!(result.is_none());
    }
}
