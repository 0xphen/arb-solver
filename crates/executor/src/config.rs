use config::{Config as ConfigLoader, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;
use std::path::PathBuf;

use super::error::Error;

#[derive(Debug, Deserialize, Clone)]
pub struct SearcherConfig {
    pub interval_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SimulatorConfig {
    pub total_nodes: usize,
    pub batch_size: usize,
    pub simulation_interval_ms: u64,
    pub rate_fluctuation_bps: f64,
    pub rebuild_limit: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub searcher: SearcherConfig,
    pub simulator: SimulatorConfig,
}

/// Loads configuration from a file and environment variables.
pub fn load_config() -> Result<Config, Error> {
    let base_path = env::current_dir().map_err(|e| {
        Error::ConfigLoadError(format!("Failed to determine current directory: {}", e))
    })?;

    let config_file_path: PathBuf = base_path
        .join("crates")
        .join("executor")
        .join("Config.toml");

    if !config_file_path.exists() {
        return Err(Error::ConfigLoadError(format!(
            "Configuration file not found at calculated path: {}",
            config_file_path.display()
        )));
    }

    let s = ConfigLoader::builder()
        .add_source(File::from(config_file_path.as_path()).required(true))
        .add_source(
            Environment::with_prefix("EXECUTOR")
                .try_parsing(true)
                .separator("_"),
        )
        .build()
        .map_err(|e| Error::ConfigLoadError(e.to_string()))?;

    let app_config: Config = s
        .try_deserialize()
        .map_err(|e| Error::ConfigLoadError(format!("Failed to deserialize config: {}", e)))?;

    Ok(app_config)
}
