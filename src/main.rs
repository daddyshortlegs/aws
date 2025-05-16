use axum::{
    routing::post,
    Router,
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    // Here you would implement the actual VM launch logic
    // For now, we'll just return a mock response
    let response = LaunchVmResponse {
        success: true,
        message: format!("VM launch request received for {} in {}", payload.name, payload.region),
        instance_id: Some("i-1234567890abcdef0".to_string()),
    };

    (StatusCode::OK, Json(response))
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