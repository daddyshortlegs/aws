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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tokio::net::TcpListener;
    use tower::ServiceExt;

    // ── helpers ───────────────────────────────────────────────────────────────

    async fn start_mock_backend(status: u16, body: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = Router::new().fallback(move || async move {
            (
                axum::http::StatusCode::from_u16(status).unwrap_or(axum::http::StatusCode::OK),
                body,
            )
        });
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        tokio::task::yield_now().await;
        port
    }

    /// Build a test app and return it alongside the shared backend-URL handle.
    /// The Router is Clone (state uses Arc internally), so callers can
    /// `.clone().oneshot(req)` multiple times against the same shared state.
    fn build_test_app() -> (Router, Arc<RwLock<Option<String>>>) {
        let backend_url: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
        let proxy_service = Arc::new(ProxyService::new(Arc::clone(&backend_url)));
        let state = AppState {
            proxy_service,
            backend_url: Arc::clone(&backend_url),
        };

        let cors = tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any);

        let app = Router::new()
            .route("/register", post(register_handler))
            .route("/launch-vm", post(proxy_handler))
            .route("/list-vms", get(proxy_handler))
            .route("/delete-vm", delete(proxy_handler))
            .fallback(proxy_handler)
            .layer(cors)
            .with_state(state);

        (app, backend_url)
    }

    async fn body_string(resp: axum::http::Response<Body>) -> String {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    // ── register handler ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_register_sets_backend_url() {
        let (app, backend_url) = build_test_app();

        let req = Request::builder()
            .method("POST")
            .uri("/register")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"ip":"127.0.0.1","port":8081}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        let url = backend_url.read().await;
        assert_eq!(url.as_deref(), Some("http://127.0.0.1:8081"));
    }

    // ── proxy routing ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_proxy_before_registration_returns_503() {
        let (app, _) = build_test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/list-vms")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_full_proxy_flow_register_then_request() {
        let port = start_mock_backend(200, "vm list").await;
        let (app, _) = build_test_app();

        // Register backend.
        let register_req = Request::builder()
            .method("POST")
            .uri("/register")
            .header("content-type", "application/json")
            .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
            .unwrap();
        app.clone().oneshot(register_req).await.unwrap();

        // Now proxy a request — should reach the mock backend.
        let proxy_req = Request::builder()
            .method("GET")
            .uri("/list-vms")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(proxy_req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        assert_eq!(body_string(resp).await, "vm list");
    }

    #[tokio::test]
    async fn test_launch_vm_route_accepts_post() {
        let port = start_mock_backend(201, "launched").await;
        let (app, _) = build_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/launch-vm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"test-vm"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_delete_vm_route_accepts_delete() {
        let port = start_mock_backend(200, "deleted").await;
        let (app, _) = build_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/delete-vm")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_fallback_route_proxies_arbitrary_path() {
        let port = start_mock_backend(200, "arbitrary").await;
        let (app, _) = build_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/some/unknown/path")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        assert_eq!(body_string(resp).await, "arbitrary");
    }

    #[tokio::test]
    async fn test_cors_headers_present_on_response() {
        let (app, _) = build_test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/list-vms")
                    .header("origin", "http://localhost:3000")
                    .header("access-control-request-method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.headers().contains_key("access-control-allow-origin"),
            "CORS allow-origin header must be present"
        );
    }
}
