use std::net::SocketAddr;
use tracing::{error, info, warn};

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::Json, routing::post, Router};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::{net::TcpListener, sync::Mutex};

    /// Starts a local Axum server and returns its base URL.
    async fn start_mock_server(app: Router) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}")
    }

    #[derive(serde::Deserialize)]
    struct RegisterPayload {
        ip: String,
        port: u16,
    }

    #[tokio::test]
    async fn test_register_success_on_first_attempt() {
        let request_count = Arc::new(AtomicUsize::new(0));
        let rc = Arc::clone(&request_count);

        let app = Router::new().route(
            "/register",
            post(move || {
                let rc = rc.clone();
                async move {
                    rc.fetch_add(1, Ordering::SeqCst);
                    axum::http::StatusCode::OK
                }
            }),
        );

        let proxy_url = start_mock_server(app).await;
        let backend_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();

        register_with_proxy(&proxy_url, backend_addr).await;

        // Should have stopped after the first successful response
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_register_sends_correct_payload() {
        let captured: Arc<Mutex<Option<RegisterPayload>>> = Arc::new(Mutex::new(None));
        let cap = Arc::clone(&captured);

        let app = Router::new().route(
            "/register",
            post(move |Json(body): Json<RegisterPayload>| {
                let cap = cap.clone();
                async move {
                    *cap.lock().await = Some(body);
                    axum::http::StatusCode::OK
                }
            }),
        );

        let proxy_url = start_mock_server(app).await;
        let backend_addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();

        register_with_proxy(&proxy_url, backend_addr).await;

        let payload = captured.lock().await;
        let payload = payload.as_ref().expect("no request received");
        assert_eq!(payload.ip, "127.0.0.1");
        assert_eq!(payload.port, 9999);
    }

    /// The mock returns 503 on the first attempt and 200 on the second.
    /// This test will take ~1 second due to the backoff sleep.
    #[tokio::test]
    async fn test_register_retries_after_non_2xx() {
        let request_count = Arc::new(AtomicUsize::new(0));
        let rc = Arc::clone(&request_count);

        let app = Router::new().route(
            "/register",
            post(move || {
                let rc = rc.clone();
                async move {
                    let prev = rc.fetch_add(1, Ordering::SeqCst);
                    if prev == 0 {
                        axum::http::StatusCode::SERVICE_UNAVAILABLE
                    } else {
                        axum::http::StatusCode::OK
                    }
                }
            }),
        );

        let proxy_url = start_mock_server(app).await;
        let backend_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();

        register_with_proxy(&proxy_url, backend_addr).await;

        // One failure + one success = two total requests
        assert_eq!(request_count.load(Ordering::SeqCst), 2);
    }
}

/// POST `{ "ip": <bound_ip>, "port": <bound_port> }` to `{proxy_url}/register`.
/// Uses the actual bound address so the proxy can reach the backend regardless
/// of which interface it is listening on.
/// Retries with exponential backoff (up to 5 attempts) so the backend can
/// start before the proxy is ready without failing fatally.
pub async fn register_with_proxy(proxy_url: &str, bound_addr: SocketAddr) {
    let ip = bound_addr.ip().to_string();
    let port = bound_addr.port();

    let url = format!("{proxy_url}/register");
    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "ip": ip, "port": port });

    let mut delay_secs = 1u64;
    for attempt in 1..=5 {
        match client.post(&url).json(&payload).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!(
                    "Registered with proxy at {} (ip={}, port={})",
                    proxy_url, ip, port
                );
                return;
            }
            Ok(resp) => {
                warn!(
                    "Registration attempt {}/5: proxy returned status {}",
                    attempt,
                    resp.status()
                );
            }
            Err(e) => {
                warn!("Registration attempt {}/5 failed: {}", attempt, e);
            }
        }

        if attempt < 5 {
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
            delay_secs = (delay_secs * 2).min(16);
        }
    }

    error!(
        "Failed to register with proxy at {} after 5 attempts",
        proxy_url
    );
}
