use axum::{
    routing::post,
    Router,
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::process::Command;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct LaunchVmRequest {
    name: String,
    instance_type: String,
    region: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LaunchVmResponse {
    success: bool,
    message: String,
    instance_id: Option<String>,
}

async fn launch_vm(
    Json(payload): Json<LaunchVmRequest>,
) -> (StatusCode, Json<LaunchVmResponse>) {
    // Get the current directory path
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let qcow2_path = current_dir.join("ubuntu.qcow2");

    // Execute QEMU command
    let output = Command::new("qemu-system-x86_64")
        .args([
            "-m", "16384",
            "-smp", "6",
            "-drive", &format!("file={}", qcow2_path.to_str().unwrap()),
            "-boot", "d",
            "-vga", "virtio",
            "-netdev", "user,id=net0,hostfwd=tcp::2222-:22",
            "-device", "e1000,netdev=net0"
        ])
        .spawn();

    match output {
        Ok(_) => {
            let response = LaunchVmResponse {
                success: true,
                message: format!("VM launch request received for {} in {}", payload.name, payload.region),
                instance_id: Some("qemu-instance".to_string()),
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let response = LaunchVmResponse {
                success: false,
                message: format!("Failed to launch VM: {}", e),
                instance_id: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build our application with a route
    let app = Router::new()
        .route("/launch-vm", post(launch_vm))
        .layer(cors);

    // Run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
} 