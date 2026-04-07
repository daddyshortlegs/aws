use crate::config::Config;
use crate::volume_db::{
    delete_volume_by_id, get_volume_by_id, list_volumes, store_volume_info, VolumeInfo,
};
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchVolumeRequest {
    pub name: String,
    pub size_gb: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchVolumeResponse {
    pub success: bool,
    pub message: String,
    pub id: Option<String>,
    pub name: Option<String>,
    pub mount_path: Option<String>,
}

pub async fn launch_volume(
    Json(payload): Json<LaunchVolumeRequest>,
) -> (StatusCode, Json<LaunchVolumeResponse>) {
    let config = Config::load().expect("Failed to load configuration");
    let volume_data_dir = &config.storage.volume_data_dir;
    let id = Uuid::new_v4().to_string();

    let img_path = volume_data_dir.join(format!("{id}.img"));
    let mount_path = volume_data_dir.join("volumes").join(&id);

    if let Err(e) = fs::create_dir_all(&mount_path).await {
        error!("Failed to create mount point {mount_path:?}: {e}");
        return error_response(format!("Failed to create mount point: {e}"));
    }

    if let Err(e) = create_sparse_image(&img_path, payload.size_gb).await {
        error!("Failed to create sparse image {img_path:?}: {e}");
        return error_response(format!("Failed to create image file: {e}"));
    }

    if let Err(e) = format_ext4(&img_path).await {
        error!("Failed to format image {img_path:?}: {e}");
        let _ = fs::remove_file(&img_path).await;
        return error_response(format!("Failed to format volume: {e}"));
    }

    if let Err(e) = mount_image(&img_path, &mount_path).await {
        error!("Failed to mount image {img_path:?} at {mount_path:?}: {e}");
        let _ = fs::remove_file(&img_path).await;
        return error_response(format!("Failed to mount volume: {e}"));
    }

    let mount_path_str = mount_path.to_string_lossy().to_string();
    let volume_info = VolumeInfo {
        id: id.clone(),
        name: payload.name.clone(),
        mount_path: mount_path_str.clone(),
        loop_device: None,
    };

    if let Err(e) = store_volume_info(volume_data_dir, &volume_info) {
        error!("Failed to store volume metadata for {id}: {e}");
        let _ = unmount_image(&mount_path).await;
        let _ = fs::remove_file(&img_path).await;
        return error_response(format!("Failed to store volume metadata: {e}"));
    }

    info!(
        "Volume {} ({}) launched, mounted at {}",
        payload.name, id, mount_path_str
    );

    (
        StatusCode::OK,
        Json(LaunchVolumeResponse {
            success: true,
            message: format!("Volume {} created and mounted", payload.name),
            id: Some(id),
            name: Some(payload.name),
            mount_path: Some(mount_path_str),
        }),
    )
}

pub async fn list_volumes_handler() -> impl IntoResponse {
    let config = Config::load().expect("Failed to load configuration");

    match list_volumes(&config.storage.volume_data_dir) {
        Ok(volumes) => (StatusCode::OK, Json(volumes)).into_response(),
        Err(e) => {
            error!("Failed to list volumes: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteVolumeRequest {
    pub id: String,
}

pub async fn delete_volume_handler(Json(payload): Json<DeleteVolumeRequest>) -> impl IntoResponse {
    info!("Deleting volume: {}", payload.id);

    let config = Config::load().expect("Failed to load configuration");
    let volume_data_dir = &config.storage.volume_data_dir;

    match get_volume_by_id(volume_data_dir, &payload.id) {
        Ok(Some(volume_info)) => {
            let mount_path = PathBuf::from(&volume_info.mount_path);
            if let Err(e) = unmount_image(&mount_path).await {
                error!("Failed to unmount {}: {e}", volume_info.mount_path);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to unmount volume: {e}"),
                )
                    .into_response();
            }

            let img_path = volume_data_dir.join(format!("{}.img", volume_info.id));
            if let Err(e) = fs::remove_file(&img_path).await {
                warn!("Could not delete image file {img_path:?}: {e}");
            } else {
                info!("Deleted image file: {img_path:?}");
            }

            if let Err(e) = delete_volume_by_id(volume_data_dir, &volume_info.id) {
                error!(
                    "Failed to delete volume metadata for {}: {e}",
                    volume_info.id
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to delete volume metadata: {e}"),
                )
                    .into_response();
            }

            (StatusCode::OK, "Volume successfully unmounted and removed").into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Volume not found").into_response(),
        Err(e) => {
            error!("Error retrieving volume info: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error retrieving volume info: {e}"),
            )
                .into_response()
        }
    }
}

async fn create_sparse_image(img_path: &Path, size_gb: u64) -> std::io::Result<()> {
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(img_path)
        .await?;
    // set_len calls ftruncate, which creates a sparse file on Linux
    file.set_len(size_gb * 1024 * 1024 * 1024).await
}

async fn format_ext4(img_path: &Path) -> std::io::Result<()> {
    let status = Command::new("mkfs.ext4").arg(img_path).status().await?;

    if !status.success() {
        return Err(std::io::Error::other(format!(
            "mkfs.ext4 exited with status {status}"
        )));
    }
    Ok(())
}

async fn mount_image(img_path: &Path, mount_point: &Path) -> std::io::Result<()> {
    let status = Command::new("mount")
        .args(["-o", "loop"])
        .arg(img_path)
        .arg(mount_point)
        .status()
        .await?;

    if !status.success() {
        return Err(std::io::Error::other(format!(
            "mount exited with status {status}"
        )));
    }
    Ok(())
}

async fn unmount_image(mount_point: &Path) -> std::io::Result<()> {
    let status = Command::new("umount").arg(mount_point).status().await?;

    if !status.success() {
        return Err(std::io::Error::other(format!(
            "umount exited with status {status}"
        )));
    }
    Ok(())
}

fn error_response(message: String) -> (StatusCode, Json<LaunchVolumeResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(LaunchVolumeResponse {
            success: false,
            message,
            id: None,
            name: None,
            mount_path: None,
        }),
    )
}
