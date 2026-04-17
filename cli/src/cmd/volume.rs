use crate::client::Client;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand)]
pub enum VolumeCommand {
    /// Create and mount a new volume
    Launch {
        /// Volume name
        #[arg(long)]
        name: String,
        /// Size in gigabytes
        #[arg(long)]
        size_gb: u64,
    },
    /// List all volumes
    List,
    /// Delete a volume
    Delete {
        /// Volume ID
        #[arg(long)]
        id: String,
    },
    /// List files in a volume
    Files {
        /// Volume ID
        #[arg(long)]
        id: String,
    },
}

#[derive(Serialize)]
struct LaunchVolumeRequest {
    name: String,
    size_gb: u64,
}

#[derive(Deserialize)]
struct LaunchVolumeResponse {
    success: bool,
    message: String,
    id: Option<String>,
    name: Option<String>,
    mount_path: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct VolumeListEntry {
    id: String,
    name: String,
    mount_path: String,
}

#[derive(Serialize)]
struct DeleteVolumeRequest {
    id: String,
}

#[derive(Deserialize, Serialize)]
struct VolumeFileEntry {
    name: String,
    is_dir: bool,
    size_bytes: u64,
    modified_secs: u64,
}

pub async fn run(cmd: VolumeCommand, client: &Client, json: bool) -> Result<(), String> {
    match cmd {
        VolumeCommand::Launch { name, size_gb } => {
            let resp: LaunchVolumeResponse = client
                .post("/launch-volume", &LaunchVolumeRequest { name, size_gb })
                .await?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": resp.success,
                        "message": resp.message,
                        "id": resp.id,
                        "name": resp.name,
                        "mount_path": resp.mount_path,
                    }))
                    .unwrap()
                );
            } else if resp.success {
                println!("Launched volume");
                if let Some(id) = &resp.id {
                    println!("  ID:         {id}");
                }
                if let Some(name) = &resp.name {
                    println!("  Name:       {name}");
                }
                if let Some(path) = &resp.mount_path {
                    println!("  Mount path: {path}");
                }
            } else {
                return Err(resp.message);
            }
        }

        VolumeCommand::List => {
            let volumes: Vec<VolumeListEntry> = client.get("/list-volumes").await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&volumes).unwrap());
            } else if volumes.is_empty() {
                println!("No volumes.");
            } else {
                println!("{:<38} {:<20} {}", "ID", "NAME", "MOUNT PATH");
                println!("{}", "-".repeat(90));
                for v in &volumes {
                    println!("{:<38} {:<20} {}", v.id, v.name, v.mount_path);
                }
            }
        }

        VolumeCommand::Delete { id } => {
            let msg = client
                .delete("/delete-volume", &DeleteVolumeRequest { id })
                .await?;
            if json {
                println!("{}", serde_json::json!({ "message": msg }));
            } else {
                println!("{msg}");
            }
        }

        VolumeCommand::Files { id } => {
            let files: Vec<VolumeFileEntry> =
                client.get(&format!("/volume-files/{id}")).await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&files).unwrap());
            } else if files.is_empty() {
                println!("Volume is empty.");
            } else {
                println!("{:<6} {:<12} {}", "TYPE", "SIZE", "NAME");
                println!("{}", "-".repeat(50));
                for f in &files {
                    let kind = if f.is_dir { "dir" } else { "file" };
                    println!("{:<6} {:<12} {}", kind, f.size_bytes, f.name);
                }
            }
        }
    }

    Ok(())
}
