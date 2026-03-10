use axum::{
    body::Body,
    extract::State,
    http::Request,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod proxy_service;

use config::Config;
use proxy_service::ProxyService;

#[derive(Clone)]
struct AppState {
    proxy_service: Arc<ProxyService>,
    backend_url: Arc<RwLock<Option<String>>>,
}

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

    // Shared backend URL — starts as None until the backend registers
    let backend_url: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

    let proxy_service = Arc::new(ProxyService::new(Arc::clone(&backend_url)));

    let state = AppState {
        proxy_service,
        backend_url,
    };

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build our application with routes
    let app = Router::new()
        .route("/register", post(register_handler))
        // Specific API endpoints with explicit methods
        .route("/launch-vm", post(proxy_handler))
        .route("/list-vms", get(proxy_handler))
        .route("/delete-vm", delete(proxy_handler))
        // Catch-all route for any other endpoints
        .fallback(proxy_handler)
        .layer(cors)
        .with_state(state);

    // Run it
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", config.proxy_port))
        .await
        .unwrap();

    tracing::info!(
        "Proxy server listening on {}",
        listener.local_addr().unwrap()
    );
    tracing::info!("Waiting for backend to register via POST /register");

    axum::serve(listener, app).await.unwrap();
}

#[derive(serde::Deserialize)]
struct RegisterRequest {
    ip: String,
    port: u16,
}

async fn register_handler(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> impl IntoResponse {
    let url = format!("http://{}:{}", body.ip, body.port);
    tracing::info!("Backend registered: {}", url);
    *state.backend_url.write().await = Some(url);
    axum::http::StatusCode::OK
}

async fn proxy_handler(State(state): State<AppState>, request: Request<Body>) -> impl IntoResponse {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    let body = Some(request.into_body());

    state
        .proxy_service
        .proxy_request(method, uri, headers, body, None)
        .await
}
