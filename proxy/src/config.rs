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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::{Mutex, OnceLock};

    // Config::load() reads process-wide env vars, so these tests must run
    // serially to avoid races with each other.
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn test_default_proxy_port() {
        let _g = env_guard();
        env::remove_var("PROXY_PORT");
        let config = Config::load().unwrap();
        assert_eq!(config.proxy_port, 8080);
    }

    #[test]
    fn test_default_log_level() {
        let _g = env_guard();
        env::remove_var("RUST_LOG");
        let config = Config::load().unwrap();
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_proxy_port_from_env() {
        let _g = env_guard();
        env::set_var("PROXY_PORT", "9090");
        let config = Config::load().unwrap();
        env::remove_var("PROXY_PORT");
        assert_eq!(config.proxy_port, 9090);
    }

    #[test]
    fn test_log_level_from_env() {
        let _g = env_guard();
        env::set_var("RUST_LOG", "debug");
        let config = Config::load().unwrap();
        env::remove_var("RUST_LOG");
        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn test_invalid_port_falls_back_to_3000() {
        let _g = env_guard();
        env::set_var("PROXY_PORT", "not_a_number");
        let config = Config::load().unwrap();
        env::remove_var("PROXY_PORT");
        // parse() fails → unwrap_or(3000)
        assert_eq!(config.proxy_port, 3000);
    }
}
