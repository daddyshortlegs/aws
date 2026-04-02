use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    pub qcow2_dir: PathBuf,
    pub metadata_dir: PathBuf,
}

#[derive(Debug, Default, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    #[default]
    User,
    Bridge,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub listen_ip: String,
    pub listen_port: u16,
    pub proxy_url: String,
    pub storage: StorageConfig,
    #[serde(default)]
    pub network_mode: NetworkMode,
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config_file = if std::env::var("CI").is_ok() {
            "config.ci".to_string()
        } else {
            let env = std::env::var("APP_ENV").unwrap_or_else(|_| "config".to_string());
            if env == "config" {
                env
            } else {
                format!("config.{env}")
            }
        };

        let config = config::Config::builder()
            .set_default("listen_ip", "127.0.0.1")?
            .set_default("listen_port", 8081)?
            .add_source(config::File::with_name(&config_file))
            .build()?;

        config.try_deserialize()
    }
}
