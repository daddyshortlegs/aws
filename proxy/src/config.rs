use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_ip: String,
    pub proxy_port: u16,
    pub log_level: String,
    /// Path to the JSON file where vm_id → backend_url mappings are persisted
    /// across proxy restarts. Defaults to `./vm-backends.json`.
    pub vm_backends_file: PathBuf,
    /// Path to the dnsmasq lease file used to resolve VM MAC addresses to IPs.
    pub lease_file: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let listen_ip = env::var("LISTEN_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
        let proxy_port = env::var("PROXY_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(3000);
        let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        let vm_backends_file = env::var("VM_BACKENDS_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./vm-backends.json"));
        let lease_file = env::var("LEASE_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/var/lib/misc/dnsmasq.leases"));

        Ok(Config {
            listen_ip,
            proxy_port,
            log_level,
            vm_backends_file,
            lease_file,
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

    #[test]
    fn test_default_vm_backends_file() {
        let _g = env_guard();
        env::remove_var("VM_BACKENDS_FILE");
        let config = Config::load().unwrap();
        assert_eq!(config.vm_backends_file, PathBuf::from("./vm-backends.json"));
    }

    #[test]
    fn test_default_lease_file() {
        let _g = env_guard();
        env::remove_var("LEASE_FILE");
        let config = Config::load().unwrap();
        assert_eq!(
            config.lease_file,
            PathBuf::from("/var/lib/misc/dnsmasq.leases")
        );
    }

    #[test]
    fn test_lease_file_from_env() {
        let _g = env_guard();
        env::set_var("LEASE_FILE", "/tmp/my.leases");
        let config = Config::load().unwrap();
        env::remove_var("LEASE_FILE");
        assert_eq!(config.lease_file, PathBuf::from("/tmp/my.leases"));
    }

    #[test]
    fn test_vm_backends_file_from_env() {
        let _g = env_guard();
        env::set_var("VM_BACKENDS_FILE", "/tmp/my-backends.json");
        let config = Config::load().unwrap();
        env::remove_var("VM_BACKENDS_FILE");
        assert_eq!(
            config.vm_backends_file,
            PathBuf::from("/tmp/my-backends.json")
        );
    }
}
