use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub id: String,
    pub name: String,
    pub mount_path: String,
    // Path to the loop device (e.g. /dev/loop0) used to mount this volume.
    // Used to detach the loop device when the volume is deleted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_device: Option<String>,
}

pub fn store_volume_info(dir: &Path, volume_info: &VolumeInfo) -> std::io::Result<()> {
    debug!("Storing volume info: {volume_info:?}");
    let file_path = create_file_path(dir, &volume_info.id);
    let json = serde_json::to_string_pretty(&volume_info)?;
    debug!("Writing volume info to: {file_path:?}");
    fs::write(file_path, json)
}

pub fn list_volumes(dir: &Path) -> std::io::Result<Vec<VolumeInfo>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut volumes = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(volume_info) = serde_json::from_str(&contents) {
                    volumes.push(volume_info);
                }
            }
        }
    }
    Ok(volumes)
}

pub fn get_volume_by_id(dir: &Path, id: &str) -> std::io::Result<Option<VolumeInfo>> {
    let file_path = create_file_path(dir, id);

    if !file_path.exists() {
        return Ok(None);
    }

    match fs::read_to_string(file_path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(volume_info) => Ok(Some(volume_info)),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        },
        Err(e) => Err(e),
    }
}

pub fn delete_volume_by_id(dir: &Path, id: &str) -> std::io::Result<()> {
    let file_path = create_file_path(dir, id);

    if file_path.exists() {
        fs::remove_file(file_path)?;
    }
    Ok(())
}

fn create_file_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_volume(id: &str, name: &str) -> VolumeInfo {
        VolumeInfo {
            id: id.to_string(),
            name: name.to_string(),
            mount_path: format!("/mnt/volumes/{id}"),
            loop_device: None,
        }
    }

    #[test]
    fn test_store_and_get_volume() {
        let dir = TempDir::new().unwrap();
        let volume = create_test_volume("vol-1", "Test Volume 1");

        store_volume_info(dir.path(), &volume).unwrap();

        let retrieved = get_volume_by_id(dir.path(), "vol-1").unwrap().unwrap();
        assert_eq!(retrieved.id, volume.id);
        assert_eq!(retrieved.name, volume.name);
        assert_eq!(retrieved.mount_path, volume.mount_path);
    }

    #[test]
    fn test_get_nonexistent_volume() {
        let dir = TempDir::new().unwrap();
        let result = get_volume_by_id(dir.path(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_volumes() {
        let dir = TempDir::new().unwrap();
        let v1 = create_test_volume("vol-2", "Test Volume 2");
        let v2 = create_test_volume("vol-3", "Test Volume 3");

        store_volume_info(dir.path(), &v1).unwrap();
        store_volume_info(dir.path(), &v2).unwrap();

        let volumes = list_volumes(dir.path()).unwrap();

        let ids: Vec<String> = volumes.iter().map(|v| v.id.clone()).collect();
        assert!(ids.contains(&v1.id));
        assert!(ids.contains(&v2.id));
    }

    #[test]
    fn test_delete_volume() {
        let dir = TempDir::new().unwrap();
        let volume = create_test_volume("vol-4", "Test Volume 4");

        store_volume_info(dir.path(), &volume).unwrap();
        delete_volume_by_id(dir.path(), "vol-4").unwrap();

        let result = get_volume_by_id(dir.path(), "vol-4").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_nonexistent_volume() {
        let dir = TempDir::new().unwrap();
        // Should succeed silently
        delete_volume_by_id(dir.path(), "nonexistent").unwrap();
    }

    #[test]
    fn test_list_volumes_empty_directory() {
        let dir = TempDir::new().unwrap();
        let volumes = list_volumes(dir.path()).unwrap();
        assert!(volumes.is_empty());
    }

    #[test]
    fn test_list_volumes_nonexistent_directory() {
        let dir = TempDir::new().unwrap();
        let nonexistent = dir.path().join("does-not-exist");
        let volumes = list_volumes(&nonexistent).unwrap();
        assert!(volumes.is_empty());
    }

    #[test]
    fn test_list_volumes_skips_non_json_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("readme.txt"), "not a volume").unwrap();

        let volume = create_test_volume("vol-5", "Test Volume 5");
        store_volume_info(dir.path(), &volume).unwrap();

        let volumes = list_volumes(dir.path()).unwrap();
        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].id, "vol-5");
    }

    #[test]
    fn test_list_volumes_skips_corrupted_json() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("bad.json"), "{ not valid json }").unwrap();

        let volume = create_test_volume("vol-6", "Test Volume 6");
        store_volume_info(dir.path(), &volume).unwrap();

        let volumes = list_volumes(dir.path()).unwrap();
        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].id, "vol-6");
    }

    #[test]
    fn test_get_volume_corrupted_json() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("bad-id.json"), "not json at all").unwrap();

        let result = get_volume_by_id(dir.path(), "bad-id");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }
}
