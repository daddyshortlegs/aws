use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod qemu;
mod register;
mod vm_db;
mod vm_service;
mod volume_db;
mod volume_service;
use vm_service::{
    delete_vm_handler, launch_vm, list_vms_handler, start_all_vms, start_vm_handler,
    stop_vm_handler,
};
use volume_service::{
    delete_volume_handler, launch_volume, list_volume_files_handler, list_volumes_handler,
};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::load().expect("Failed to load configuration");
    tracing::info!("Loaded configuration: {:?}", config);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/launch-vm", post(launch_vm))
        .route("/list-vms", get(list_vms_handler))
        .route("/delete-vm", delete(delete_vm_handler))
        .route("/stop-vm", post(stop_vm_handler))
        .route("/start-vm", post(start_vm_handler))
        .route("/launch-volume", post(launch_volume))
        .route("/list-volumes", get(list_volumes_handler))
        .route("/delete-volume", delete(delete_volume_handler))
        .route("/volume-files/:id", get(list_volume_files_handler))
        .layer(cors);

    start_all_vms().await;

    // Bind first so we know the actual port before registering
    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.listen_ip, config.listen_port))
            .await
            .unwrap();
    let bound_addr = listener.local_addr().unwrap();
    tracing::info!("listening on {}", bound_addr);

    // Announce ourselves to the proxy asynchronously so startup is not blocked
    let proxy_url = config.proxy_url.clone();
    tokio::spawn(async move {
        register::register_with_proxy(&proxy_url, bound_addr).await;
    });

    axum::serve(listener, app).await.unwrap();
}
