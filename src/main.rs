use axum::{
    routing::post,
    Router,
};
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod vm_launcher;
use vm_launcher::launch_vm;

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