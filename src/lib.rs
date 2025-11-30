pub mod builder;
pub mod config;
pub mod database;
pub mod models;
pub mod rpc;
pub mod script;
pub mod signer;

use chrono::Utc;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

pub use builder::{TransactionBuilder, TxBuilderError};
pub use config::{ConfigError, ZcashConfig};
pub use models::*;
pub use rpc::{RpcClientError, ZcashRpcClient};
pub use script::{HTLCScriptBuilder, HTLCScriptError};
pub use signer::{SignerError, TransactionSigner};

use crate::database::{Database, DatabaseError};

pub struct ZcashHTLCClient {
    config: ZcashConfig,
    database: Arc<Database>,
    rpc_client: ZcashRpcClient,
    tx_builder: TransactionBuilder,
    signer: TransactionSigner,
    script_builder: HTLCScriptBuilder,
}

impl ZcashHTLCClient {
    /// Create new client from configuration
    pub fn new(config: ZcashConfig, database: Arc<Database>) -> Self {
        let rpc_client = ZcashRpcClient::new(
            config.rpc_url.clone(),
            config.rpc_user.clone(),
            config.rpc_password.clone(),
            config.network,
        );

        let rpc_client = if let Some(explorer) = &config.explorer_api {
            rpc_client.with_custom_explorer(explorer.clone())
        } else {
            rpc_client
        };

        let tx_builder = TransactionBuilder::new(config.network);
        let script_builder = HTLCScriptBuilder::new(config.network);
        let signer = TransactionSigner::new(script_builder.clone());

        Self {
            config,
            database,
            rpc_client,
            tx_builder,
            signer,
            script_builder: script_builder.clone(),
        }
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self, HTLCClientError> {
        dotenv::dotenv().ok();
        
        let config = ZcashConfig::from_env()?;
        let database = Arc::new(Database::from_env()?);

        Ok(Self::new(config, database))
    }

    // ==================== HTLC Operations ====================

    /// Create a new HTLC
    pub async fn create_htlc(
        &self,
        params: HTLCParams,
        funding_utxos: Vec<UTXO>,
        change_address: &str,
        funding_privkeys: Vec<&str>,
    ) -> Result<HTLCCreationResult, HTLCClientError> {
        info!("🔨 Creating HTLC for {} ZEC", params.amount);

        // Build HTLC transaction
        let (tx, redeem_script) = self
            .tx_builder
            .build_htlc_tx(&params, funding_utxos.clone(), change_address)?;

        // Generate P2SH address
        let p2sh_address = self.script_builder.script_to_p2sh_address(&redeem_script)?;
        info!("📍 P2SH address: {}", p2sh_address);

        // Build script pubkeys for signing
        let input_scripts: Vec<_> = funding_utxos
            .iter()
            .map(|utxo| {
                hex::decode(&utxo.script_pubkey)
                    .map(bitcoin::blockdata::script::Script::from)
                    .map_err(|_| HTLCClientError::InvalidScript)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Sign transaction
        let signed_tx = self
            .signer
            .sign_htlc_creation(tx, input_scripts, funding_privkeys)?;

        let tx_hex = self.tx_builder.serialize_tx(&signed_tx);
        let htlc_id = Uuid::new_v4().to_string();

        // Create database record
        let htlc = ZcashHTLC {
            id: htlc_id.clone(),
            txid: None,
            p2sh_address: p2sh_address.clone(),
            hash_lock: params.hash_lock.clone(),
            secret: None,
            timelock: params.timelock,
            recipient_pubkey: params.recipient_pubkey.clone(),
            refund_pubkey: params.refund_pubkey.clone(),
            amount: params.amount.clone(),
            network: self.config.network,
            state: HTLCState::Pending,
            vout: None,
            script_hex: hex::encode(redeem_script.as_bytes()),
            redeem_script_hex: hex::encode(redeem_script.as_bytes()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.database.create_htlc(&htlc)?;

        // Create operation record
        let operation_id = Uuid::new_v4().to_string();
        let operation = HTLCOperation {
            id: operation_id,
            htlc_id: htlc_id.clone(),
            operation_type: HTLCOperationType::Create,
            txid: None,
            raw_tx_hex: Some(tx_hex.clone()),
            signed_tx_hex: Some(tx_hex.clone()),
            broadcast_at: None,
            confirmed_at: None,
            block_height: None,
            status: OperationStatus::Signed,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.database.create_operation(&operation)?;

        // Broadcast transaction
        let txid = self.rpc_client.send_raw_transaction(&tx_hex).await?;
        
        // Update database
        self.database.update_htlc_txid(&htlc_id, &txid, 0)?;
        self.database.update_operation_broadcast(&operation.id, &txid)?;

        info!("✅ HTLC created with txid: {}", txid);

        Ok(HTLCCreationResult {
            htlc_id,
            txid,
            p2sh_address,
            redeem_script: hex::encode(redeem_script.as_bytes()),
        })
    }

    /// Redeem an HTLC with the secret
    pub async fn redeem_htlc(
        &self,
        htlc_id: &str,
        secret: &str,
        recipient_address: &str,
        recipient_privkey: &str,
    ) -> Result<String, HTLCClientError> {
        info!("🔓 Redeeming HTLC: {}", htlc_id);

        // Load HTLC from database
        let htlc = self.database.get_htlc_by_id(htlc_id)?;

        // Verify secret
        if !self.script_builder.verify_secret(secret, &htlc.hash_lock) {
            return Err(HTLCClientError::InvalidSecret);
        }

        let txid = htlc.txid.ok_or(HTLCClientError::HTLCNotLocked)?;
        let vout = htlc.vout.ok_or(HTLCClientError::HTLCNotLocked)?;

        // Decode redeem script
        let redeem_script_bytes = hex::decode(&htlc.redeem_script_hex)
            .map_err(|_| HTLCClientError::InvalidScript)?;
        let redeem_script = bitcoin::blockdata::script::Script::from(redeem_script_bytes);

        // Build redeem transaction
        let tx = self.tx_builder.build_redeem_tx(
            &txid,
            vout,
            &htlc.amount,
            secret,
            &redeem_script,
            recipient_address,
        )?;

        // Sign transaction
        let signed_tx = self.signer.sign_htlc_redeem(
            tx,
            0,
            &redeem_script,
            secret,
            recipient_privkey,
        )?;

        let tx_hex = self.tx_builder.serialize_tx(&signed_tx);

        // Create operation record
        let operation_id = Uuid::new_v4().to_string();
        let operation = HTLCOperation {
            id: operation_id.clone(),
            htlc_id: htlc_id.to_string(),
            operation_type: HTLCOperationType::Redeem,
            txid: None,
            raw_tx_hex: Some(tx_hex.clone()),
            signed_tx_hex: Some(tx_hex.clone()),
            broadcast_at: None,
            confirmed_at: None,
            block_height: None,
            status: OperationStatus::Signed,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.database.create_operation(&operation)?;

        // Broadcast transaction
        let redeem_txid = self.rpc_client.send_raw_transaction(&tx_hex).await?;

        // Update database
        self.database.update_htlc_state(htlc_id, HTLCState::Redeemed)?;
        self.database.update_htlc_secret(htlc_id, secret)?;
        self.database.update_operation_broadcast(&operation_id, &redeem_txid)?;

        info!("✅ HTLC redeemed with txid: {}", redeem_txid);

        Ok(redeem_txid)
    }

    /// Refund an HTLC after timelock expiry
    pub async fn refund_htlc(
        &self,
        htlc_id: &str,
        refund_address: &str,
        refund_privkey: &str,
    ) -> Result<String, HTLCClientError> {
        info!("♻️ Refunding HTLC: {}", htlc_id);

        // Load HTLC from database
        let htlc = self.database.get_htlc_by_id(htlc_id)?;

        let txid = htlc.txid.ok_or(HTLCClientError::HTLCNotLocked)?;
        let vout = htlc.vout.ok_or(HTLCClientError::HTLCNotLocked)?;

        // Check timelock
        let current_block = self.rpc_client.get_block_count().await?;
        if current_block < htlc.timelock {
            return Err(HTLCClientError::TimelockNotExpired {
                current: current_block,
                required: htlc.timelock,
            });
        }

        // Decode redeem script
        let redeem_script_bytes = hex::decode(&htlc.redeem_script_hex)
            .map_err(|_| HTLCClientError::InvalidScript)?;
        let redeem_script = bitcoin::blockdata::script::Script::from(redeem_script_bytes);

        // Build refund transaction
        let tx = self.tx_builder.build_refund_tx(
            &txid,
            vout,
            &htlc.amount,
            htlc.timelock,
            &redeem_script,
            refund_address,
        )?;

        // Sign transaction
        let signed_tx = self.signer.sign_htlc_refund(tx, 0, &redeem_script, refund_privkey)?;

        let tx_hex = self.tx_builder.serialize_tx(&signed_tx);

        // Create operation record
        let operation_id = Uuid::new_v4().to_string();
        let operation = HTLCOperation {
            id: operation_id.clone(),
            htlc_id: htlc_id.to_string(),
            operation_type: HTLCOperationType::Refund,
            txid: None,
            raw_tx_hex: Some(tx_hex.clone()),
            signed_tx_hex: Some(tx_hex.clone()),
            broadcast_at: None,
            confirmed_at: None,
            block_height: None,
            status: OperationStatus::Signed,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.database.create_operation(&operation)?;

        // Broadcast transaction
        let refund_txid = self.rpc_client.send_raw_transaction(&tx_hex).await?;

        // Update database
        self.database.update_htlc_state(htlc_id, HTLCState::Refunded)?;
        self.database.update_operation_broadcast(&operation_id, &refund_txid)?;

        info!("✅ HTLC refunded with txid: {}", refund_txid);

        Ok(refund_txid)
    }

    // ==================== Query Methods ====================

    /// Get HTLC by ID
    pub fn get_htlc(&self, htlc_id: &str) -> Result<ZcashHTLC, HTLCClientError> {
        Ok(self.database.get_htlc_by_id(htlc_id)?)
    }

    /// Get UTXOs for address
    pub async fn get_utxos(&self, address: &str) -> Result<Vec<UTXO>, HTLCClientError> {
        Ok(self.rpc_client.get_utxos(address).await?)
    }

    /// Get address balance
    pub async fn get_balance(&self, address: &str) -> Result<String, HTLCClientError> {
        Ok(self.rpc_client.get_balance(address).await?)
    }

    /// Wait for transaction confirmation
    pub async fn wait_for_confirmation(
        &self,
        txid: &str,
        confirmations: u32,
    ) -> Result<u32, HTLCClientError> {
        Ok(self
            .rpc_client
            .wait_for_confirmations(txid, confirmations, 60)
            .await?)
    }

    // ==================== Key Management ====================

    /// Generate new private key
    pub fn generate_privkey(&self) -> String {
        self.signer.generate_privkey()
    }

    /// Derive public key from private key
    pub fn derive_pubkey(&self, privkey: &str) -> Result<String, HTLCClientError> {
        Ok(self.signer.derive_pubkey(privkey)?)
    }

    /// Generate hash lock from secret
    pub fn generate_hash_lock(&self, secret: &str) -> String {
        self.signer.generate_hash_lock(secret)
    }

    // ==================== Utilities ====================

    /// Get current network
    pub fn network(&self) -> ZcashNetwork {
        self.config.network
    }

    /// Get database reference
    pub fn database(&self) -> &Database {
        &self.database
    }
}

// ==================== Error Types ====================

#[derive(Debug, thiserror::Error)]
pub enum HTLCClientError {
    #[error("Config error: {0}")]
    ConfigError(#[from] ConfigError),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),
    
    #[error("RPC error: {0}")]
    RpcError(#[from] RpcClientError),
    
    #[error("Transaction builder error: {0}")]
    TxBuilderError(#[from] TxBuilderError),
    
    #[error("Script error: {0}")]
    ScriptError(#[from] HTLCScriptError),
    
    #[error("Signer error: {0}")]
    SignerError(#[from] SignerError),
    
    #[error("Invalid secret for hash lock")]
    InvalidSecret,
    
    #[error("HTLC not locked (missing txid or vout)")]
    HTLCNotLocked,
    
    #[error("Invalid script format")]
    InvalidScript,
    
    #[error("Timelock not expired (current: {current}, required: {required})")]
    TimelockNotExpired { current: u64, required: u64 },
}