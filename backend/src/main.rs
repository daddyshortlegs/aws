use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use std::io;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod vm_db;
mod config;
mod vm_service;
use vm_service::{delete_vm_handler, launch_vm, list_vms_handler};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = config::Config::load().expect("Failed to load configuration");
    tracing::info!("Loaded configuration: {:?}", config);

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build our application with a route
    let app = Router::new()
        .route("/launch-vm", post(launch_vm))
        .route("/list-vms", get(list_vms_handler))
        .route("/delete-vm", delete(delete_vm_handler))
        .layer(cors);

    // Run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
