use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use crate::{ZcashNetwork, RelayerConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZcashConfig {
    pub network: ZcashNetwork,
    pub rpc_url: String,
    pub rpc_user: Option<String>,
    pub rpc_password: Option<String>,
    pub explorer_api: Option<String>,
    pub database_url: String,
    pub database_max_connections: u32,
    pub relayer: Option<RelayerConfig>,
}

impl ZcashConfig {
    pub fn new(network: ZcashNetwork, rpc_url: String, database_url: String) -> Self {
        Self {
            network,
            rpc_url,
            rpc_user: None,
            rpc_password: None,
            explorer_api: None,
            database_url,
            database_max_connections: 10,
            relayer: None,
        }
    }

    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::InvalidConfig(format!("Failed to read config file: {}", e)))?;
        
        toml::from_str(&content)
            .map_err(|e| ConfigError::InvalidConfig(format!("Failed to parse TOML: {}", e)))
    }

    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::InvalidConfig(format!("Failed to read config file: {}", e)))?;
        
        serde_json::from_str(&content)
            .map_err(|e| ConfigError::InvalidConfig(format!("Failed to parse JSON: {}", e)))
    }

    pub fn with_auth(mut self, user: String, password: String) -> Self {
        self.rpc_user = Some(user);
        self.rpc_password = Some(password);
        self
    }

    pub fn with_explorer(mut self, api_url: String) -> Self {
        self.explorer_api = Some(api_url);
        self
    }

    pub fn with_max_connections(mut self, max: u32) -> Self {
        self.database_max_connections = max;
        self
    }
    
    pub fn with_relayer(mut self, relayer: RelayerConfig) -> Self {
        self.relayer = Some(relayer);
        self
    }

    pub fn from_default_locations() -> Result<Self, ConfigError> {
        let possible_paths = vec![
            "./zcash-config.toml",
            "./zcash-config.json",
            "../zcash-config.toml",
            "../zcash-config.json",
        ];

        for path in possible_paths {
            if Path::new(path).exists() {
                return if path.ends_with(".json") {
                    Self::from_json_file(path)
                } else {
                    Self::from_toml_file(path)
                };
            }
        }

        Err(ConfigError::InvalidConfig(
            "No config file found. Create zcash-config.toml in project root".to_string()
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}