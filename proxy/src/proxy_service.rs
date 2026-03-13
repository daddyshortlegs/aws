use axum::{
    body::Body,
    extract::Query,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::IntoResponse,
};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

pub struct ProxyService {
    client: Client,
    pub backend_url: Arc<RwLock<Option<String>>>,
}

impl ProxyService {
    pub fn new(backend_url: Arc<RwLock<Option<String>>>) -> Self {
        let client = Client::new();
        Self {
            client,
            backend_url,
        }
    }

    pub async fn proxy_request(
        &self,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Option<Body>,
        query: Option<Query<HashMap<String, String>>>,
    ) -> impl IntoResponse {
        let path = uri.path();

        let url = {
            let guard = self.backend_url.read().await;
            match guard.as_ref() {
                Some(u) => u.clone(),
                None => {
                    return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "Backend not yet registered",
                    )
                        .into_response();
                }
            }
        };

        let backend_url = format!("{url}{path}");
        info!("Proxying {} {} -> {}", method, path, backend_url);

        // Convert Axum Method to Reqwest Method
        let reqwest_method = match method.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            _ => reqwest::Method::GET, // Default fallback
        };

        // Build the request
        let mut request_builder = self.client.request(reqwest_method, &backend_url);

        // Add headers (excluding host and connection headers)
        for (key, value) in headers.iter() {
            if key != "host" && key != "connection" {
                // Convert Axum header types to Reqwest header types
                if let (Ok(name), Ok(val)) = (
                    reqwest::header::HeaderName::from_bytes(key.as_str().as_bytes()),
                    reqwest::header::HeaderValue::from_bytes(value.as_bytes()),
                ) {
                    request_builder = request_builder.header(name, val);
                }
            }
        }

        // Add query parameters if present
        if let Some(query) = query {
            request_builder = request_builder.query(&query.0);
        }

        // Add body for POST/PUT/PATCH requests
        let request = if let Some(body) = body {
            // Convert Axum Body to bytes first
            let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Failed to read body: {}", e);
                    return (StatusCode::BAD_REQUEST, "Failed to read request body")
                        .into_response();
                }
            };

            // Convert to Reqwest Body
            let reqwest_body = reqwest::Body::from(body_bytes);
            request_builder.body(reqwest_body).build().unwrap()
        } else {
            request_builder.build().unwrap()
        };

        // Execute the request
        match self.client.execute(request).await {
            Ok(response) => {
                let status = response.status();
                let headers = response.headers().clone();
                let body_bytes = response.bytes().await.unwrap_or_default();

                info!("Backend response: {}", status);

                // Convert Reqwest status to Axum status
                let axum_status = StatusCode::from_u16(status.as_u16())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                // Convert response back to axum response
                let mut response_builder = axum::http::Response::builder().status(axum_status);

                // Copy headers from backend response
                for (key, value) in headers.iter() {
                    if key != "transfer-encoding" {
                        // Convert Reqwest header types to Axum header types
                        if let (Ok(name), Ok(val)) = (
                            axum::http::HeaderName::from_bytes(key.as_str().as_bytes()),
                            axum::http::HeaderValue::from_bytes(value.as_bytes()),
                        ) {
                            response_builder = response_builder.header(name, val);
                        }
                    }
                }

                response_builder
                    .body(Body::from(body_bytes))
                    .unwrap()
                    .into_response()
            }
            Err(e) => {
                error!("Proxy request failed: {}", e);
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to proxy request: {e}"),
                )
                    .into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProxyService;
    use axum::{
        body::Body,
        extract::Query,
        http::{HeaderMap, Method, StatusCode, Uri},
        response::IntoResponse,
        Router,
    };
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::net::TcpListener;
    use tokio::sync::{Mutex, RwLock};

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Spin up a mock backend that replies with a fixed status + body for every request.
    async fn start_mock_backend(status: u16, body: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = Router::new().fallback(move || async move {
            (StatusCode::from_u16(status).unwrap_or(StatusCode::OK), body)
        });
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        tokio::task::yield_now().await;
        port
    }

    /// Spin up a mock backend that echoes the request body back in the response.
    async fn start_body_echo_backend() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = Router::new().fallback(|body: axum::body::Bytes| async move { body });
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        tokio::task::yield_now().await;
        port
    }

    /// Spin up a mock backend that echoes the request path back in the response.
    async fn start_path_echo_backend() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = Router::new().fallback(|uri: Uri| async move { uri.path().to_string() });
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        tokio::task::yield_now().await;
        port
    }

    /// Spin up a mock backend that stores every incoming header in the returned Arc.
    async fn start_header_capture_backend() -> (u16, Arc<Mutex<Vec<(String, String)>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let captured: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(vec![]));
        let cap = captured.clone();
        let app = Router::new().fallback(move |headers: HeaderMap| {
            let cap = cap.clone();
            async move {
                *cap.lock().await = headers
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                StatusCode::OK
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        tokio::task::yield_now().await;
        (port, captured)
    }

    fn make_service(backend_url: Option<String>) -> ProxyService {
        ProxyService::new(Arc::new(RwLock::new(backend_url)))
    }

    fn service_for_port(port: u16) -> ProxyService {
        make_service(Some(format!("http://127.0.0.1:{port}")))
    }

    async fn body_string(resp: axum::http::Response<Body>) -> String {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    // ── no-backend cases ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_no_backend_returns_service_unavailable() {
        let svc = make_service(None);
        let resp = svc
            .proxy_request(
                Method::GET,
                "/".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_no_backend_body_says_not_registered() {
        let svc = make_service(None);
        let resp = svc
            .proxy_request(
                Method::GET,
                "/".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert!(body_string(resp)
            .await
            .contains("Backend not yet registered"));
    }

    // ── HTTP method forwarding ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_request_proxied() {
        let port = start_mock_backend(200, "get ok").await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::GET,
                "/path".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_string(resp).await, "get ok");
    }

    #[tokio::test]
    async fn test_post_body_forwarded() {
        let port = start_body_echo_backend().await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::POST,
                "/submit".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                Some(Body::from("hello world")),
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_string(resp).await, "hello world");
    }

    #[tokio::test]
    async fn test_put_body_forwarded() {
        let port = start_body_echo_backend().await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::PUT,
                "/update".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                Some(Body::from("update payload")),
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_string(resp).await, "update payload");
    }

    #[tokio::test]
    async fn test_patch_body_forwarded() {
        let port = start_body_echo_backend().await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::PATCH,
                "/patch".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                Some(Body::from("patch data")),
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_string(resp).await, "patch data");
    }

    #[tokio::test]
    async fn test_delete_request_proxied() {
        let port = start_mock_backend(200, "deleted").await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::DELETE,
                "/item/1".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_head_request_proxied() {
        let port = start_mock_backend(200, "").await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::HEAD,
                "/".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_options_request_proxied() {
        let port = start_mock_backend(200, "").await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::OPTIONS,
                "/".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_unknown_method_falls_back_to_get() {
        // An unrecognised method string falls back to GET rather than erroring.
        let port = start_mock_backend(200, "ok").await;
        let svc = service_for_port(port);
        let unknown = Method::from_bytes(b"FOOBAR").unwrap();
        let resp = svc
            .proxy_request(
                unknown,
                "/".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // ── error cases ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_unreachable_backend_returns_bad_gateway() {
        // Port 1 is reserved and nothing will be listening there.
        let svc = make_service(Some("http://127.0.0.1:1".to_string()));
        let resp = svc
            .proxy_request(
                Method::GET,
                "/".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }

    // ── routing / URL construction ────────────────────────────────────────────

    #[tokio::test]
    async fn test_request_path_forwarded_to_backend() {
        let port = start_path_echo_backend().await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::GET,
                "/api/vms/abc-123".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_string(resp).await, "/api/vms/abc-123");
    }

    #[tokio::test]
    async fn test_query_params_forwarded_when_provided() {
        // Query params passed via the Query argument are added to the backend URL.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app =
            Router::new().fallback(|Query(params): Query<HashMap<String, String>>| async move {
                serde_json::to_string(&params).unwrap()
            });
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        tokio::task::yield_now().await;

        let svc = service_for_port(port);
        let mut params = HashMap::new();
        params.insert("filter".to_string(), "running".to_string());

        let resp = svc
            .proxy_request(
                Method::GET,
                "/list-vms".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                Some(Query(params)),
            )
            .await
            .into_response();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(body.contains("filter") && body.contains("running"));
    }

    // ── header forwarding ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_host_header_not_forwarded_with_original_value() {
        let (port, captured) = start_header_capture_backend().await;
        let svc = service_for_port(port);
        let mut headers = HeaderMap::new();
        headers.insert("host", "proxy.example.com".parse().unwrap());

        let _ = svc
            .proxy_request(
                Method::GET,
                "/".parse::<Uri>().unwrap(),
                headers,
                None,
                None,
            )
            .await
            .into_response();

        let captured = captured.lock().await;
        // Reqwest will add its own host header pointing at the backend, but the
        // original proxy hostname must not appear.
        let has_proxy_host = captured
            .iter()
            .any(|(k, v)| k == "host" && v == "proxy.example.com");
        assert!(!has_proxy_host);
    }

    #[tokio::test]
    async fn test_connection_header_not_forwarded() {
        let (port, captured) = start_header_capture_backend().await;
        let svc = service_for_port(port);
        let mut headers = HeaderMap::new();
        headers.insert("connection", "keep-alive".parse().unwrap());

        let _ = svc
            .proxy_request(
                Method::GET,
                "/".parse::<Uri>().unwrap(),
                headers,
                None,
                None,
            )
            .await
            .into_response();

        let captured = captured.lock().await;
        let has_connection = captured.iter().any(|(k, _)| k == "connection");
        assert!(!has_connection);
    }

    #[tokio::test]
    async fn test_custom_headers_forwarded() {
        let (port, captured) = start_header_capture_backend().await;
        let svc = service_for_port(port);
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "test-id-42".parse().unwrap());

        let _ = svc
            .proxy_request(
                Method::GET,
                "/".parse::<Uri>().unwrap(),
                headers,
                None,
                None,
            )
            .await
            .into_response();

        let captured = captured.lock().await;
        let has_custom = captured
            .iter()
            .any(|(k, v)| k == "x-request-id" && v == "test-id-42");
        assert!(has_custom);
    }

    // ── response passthrough ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_backend_404_preserved() {
        let port = start_mock_backend(404, "not found").await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::GET,
                "/missing".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_backend_500_preserved() {
        let port = start_mock_backend(500, "server error").await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::GET,
                "/boom".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_response_body_preserved() {
        let port = start_mock_backend(200, "exact body content").await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::GET,
                "/".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(body_string(resp).await, "exact body content");
    }

    #[tokio::test]
    async fn test_custom_response_headers_forwarded() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = Router::new().fallback(|| async {
            axum::http::Response::builder()
                .header("x-custom", "preserved")
                .body(Body::from("body"))
                .unwrap()
        });
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        tokio::task::yield_now().await;

        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::GET,
                "/".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();

        assert!(resp.headers().contains_key("x-custom"));
    }

    #[tokio::test]
    async fn test_transfer_encoding_absent_from_proxied_response() {
        // transfer-encoding is a hop-by-hop header; it must never appear in the
        // response the proxy delivers to the client.
        let port = start_mock_backend(200, "body").await;
        let svc = service_for_port(port);
        let resp = svc
            .proxy_request(
                Method::GET,
                "/".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();

        assert!(!resp.headers().contains_key("transfer-encoding"));
    }

    // ── dynamic backend registration ──────────────────────────────────────────

    #[tokio::test]
    async fn test_backend_url_updated_dynamically() {
        let backend_url: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
        let svc = ProxyService::new(Arc::clone(&backend_url));

        // Before registration the proxy must refuse requests.
        let resp = svc
            .proxy_request(
                Method::GET,
                "/".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        // Simulate backend registering itself.
        let port = start_mock_backend(200, "registered").await;
        *backend_url.write().await = Some(format!("http://127.0.0.1:{port}"));

        let resp = svc
            .proxy_request(
                Method::GET,
                "/".parse::<Uri>().unwrap(),
                HeaderMap::new(),
                None,
                None,
            )
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
