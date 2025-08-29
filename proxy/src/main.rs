use axum::{
    body::Body,
    extract::State,
    http::Request,
    response::IntoResponse,
    routing::{get, post, delete},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod proxy_service;

use config::Config;
use proxy_service::ProxyService;

#[tokio::main]
async fn main() {
    // Load configuration
    let config = Config::load().expect("Failed to load configuration");
    
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&config.log_level))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting proxy server with config: {:?}", config);

    // Create proxy service
    let proxy_service = Arc::new(ProxyService::new(config.backend_url.clone()));

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build our application with routes
    let app = Router::new()
        // Specific API endpoints with explicit methods
        .route("/launch-vm", post(proxy_handler))
        .route("/list-vms", get(proxy_handler))
        .route("/delete-vm", delete(proxy_handler))
        .route("/ws", get(websocket_handler))
        // Catch-all route for any other endpoints
        .fallback(proxy_handler)
        .layer(cors)
        .with_state(proxy_service);

    // Run it
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", config.proxy_port))
        .await
        .unwrap();
    
    tracing::info!("Proxy server listening on {}", listener.local_addr().unwrap());
    tracing::info!("Proxying requests to backend at {}", config.backend_url);
    
    axum::serve(listener, app).await.unwrap();
}

async fn proxy_handler(
    State(proxy_service): State<Arc<ProxyService>>,
    request: Request<Body>,
) -> impl IntoResponse {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    let body = Some(request.into_body());
    
    proxy_service.proxy_request(method, uri, headers, body, None).await
}

async fn websocket_handler() -> &'static str {
    "WebSocket endpoint - connect directly to backend"
}
