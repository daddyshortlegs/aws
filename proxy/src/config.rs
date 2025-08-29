use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub backend_url: String,
    pub proxy_port: u16,
    pub log_level: String,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let backend_url = env::var("BACKEND_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
        let proxy_port = env::var("PROXY_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .unwrap_or(3000);
        let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".into());

        Ok(Config {
            backend_url,
            proxy_port,
            log_level,
        })
    }
}
