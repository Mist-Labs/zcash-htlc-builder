use serde::{Deserialize, Serialize};
use std::env;

use crate::ZcashNetwork;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZcashConfig {
    pub network: ZcashNetwork,
    pub rpc_url: String,
    pub rpc_user: Option<String>,
    pub rpc_password: Option<String>,
    pub explorer_api: Option<String>,
    pub database_url: String,
    pub database_max_connections: u32,
}

impl ZcashConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let network = env::var("ZCASH_NETWORK")
            .unwrap_or_else(|_| "testnet".to_string());
        
        let network = ZcashNetwork::from_str(&network);

        let rpc_url = env::var("ZCASH_RPC_URL")
            .map_err(|_| ConfigError::MissingEnvVar("ZCASH_RPC_URL"))?;

        let rpc_user = env::var("ZCASH_RPC_USER").ok();
        let rpc_password = env::var("ZCASH_RPC_PASSWORD").ok();
        let explorer_api = env::var("ZCASH_EXPLORER_API").ok();

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingEnvVar("DATABASE_URL"))?;

        let database_max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        Ok(Self {
            network,
            rpc_url,
            rpc_user,
            rpc_password,
            explorer_api,
            database_url,
            database_max_connections,
        })
    }

    pub fn new(
        network: ZcashNetwork,
        rpc_url: String,
        database_url: String,
    ) -> Self {
        Self {
            network,
            rpc_url,
            rpc_user: None,
            rpc_password: None,
            explorer_api: None,
            database_url,
            database_max_connections: 10,
        }
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
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(&'static str),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}