use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    pub qcow2_dir: PathBuf,
    pub metadata_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub storage: StorageConfig,
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config"))
            .build()?;

        config.try_deserialize()
    }

    pub fn get_vms_dir() -> PathBuf {
        let config = Self::load().expect("Failed to load configuration");
        config.storage.metadata_dir
    }
} 