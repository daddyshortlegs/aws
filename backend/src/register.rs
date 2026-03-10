use std::net::SocketAddr;
use tracing::{error, info, warn};

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
