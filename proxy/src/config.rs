use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub proxy_port: u16,
    pub log_level: String,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let proxy_port = env::var("PROXY_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(3000);
        let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".into());

        Ok(Config {
            proxy_port,
            log_level,
        })
    }
}
