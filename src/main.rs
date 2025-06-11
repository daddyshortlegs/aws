use axum::{routing::{post, get}, Router, response::IntoResponse, http::StatusCode};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::io;

mod vm_launcher;
mod vm_db;
use vm_launcher::launch_vm;
use vm_db::list_vms;

async fn list_vms_handler() -> impl IntoResponse {
    match list_vms() {
        Ok(vms) => (StatusCode::OK, axum::Json(vms)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
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
        .route("/list-vms", get(list_vms_handler))
        .layer(cors);

    // Run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
