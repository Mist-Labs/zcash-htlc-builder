pub mod schema;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i16)]
pub enum HTLCState {
    Pending = 0,
    Locked = 1,
    Redeemed = 2,
    Refunded = 3,
    Expired = 4,
    Failed = 5,
}

impl HTLCState {
    pub fn from_i16(value: i16) -> Self {
        match value {
            0 => HTLCState::Pending,
            1 => HTLCState::Locked,
            2 => HTLCState::Redeemed,
            3 => HTLCState::Refunded,
            4 => HTLCState::Expired,
            5 => HTLCState::Failed,
            _ => HTLCState::Pending,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HTLCState::Pending => "pending",
            HTLCState::Locked => "locked",
            HTLCState::Redeemed => "redeemed",
            HTLCState::Refunded => "refunded",
            HTLCState::Expired => "expired",
            HTLCState::Failed => "failed"
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HTLCOperationType {
    Create,
    Redeem,
    Refund,
}

impl HTLCOperationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HTLCOperationType::Create => "create",
            HTLCOperationType::Redeem => "redeem",
            HTLCOperationType::Refund => "refund",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "create" => HTLCOperationType::Create,
            "redeem" => HTLCOperationType::Redeem,
            "refund" => HTLCOperationType::Refund,
            _ => HTLCOperationType::Create,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZcashHTLC {
    pub id: String,
    pub txid: Option<String>,
    pub p2sh_address: String,
    pub hash_lock: String,
    pub secret: Option<String>,
    pub timelock: u64,
    pub recipient_pubkey: String,
    pub refund_pubkey: String,
    pub amount: String,
    pub network: ZcashNetwork,
    pub state: HTLCState,
    pub vout: Option<u32>,
    pub script_hex: String,
    pub redeem_script_hex: String,
    pub signed_redeem_tx: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTLCOperation {
    pub id: String,
    pub htlc_id: String,
    pub operation_type: HTLCOperationType,
    pub txid: Option<String>,
    pub raw_tx_hex: Option<String>,
    pub signed_tx_hex: Option<String>,
    pub broadcast_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub block_height: Option<u64>,
    pub status: OperationStatus,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    Pending,
    Signed,
    Broadcast,
    Confirmed,
    Failed,
}

impl OperationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationStatus::Pending => "pending",
            OperationStatus::Signed => "signed",
            OperationStatus::Broadcast => "broadcast",
            OperationStatus::Confirmed => "confirmed",
            OperationStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => OperationStatus::Pending,
            "signed" => OperationStatus::Signed,
            "broadcast" => OperationStatus::Broadcast,
            "confirmed" => OperationStatus::Confirmed,
            "failed" => OperationStatus::Failed,
            _ => OperationStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZcashNetwork {
    Mainnet,
    Testnet,
}

impl ZcashNetwork {
    pub fn as_str(&self) -> &'static str {
        match self {
            ZcashNetwork::Mainnet => "mainnet",
            ZcashNetwork::Testnet => "testnet",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "mainnet" => ZcashNetwork::Mainnet,
            "testnet" => ZcashNetwork::Testnet,
            _ => ZcashNetwork::Testnet,
        }
    }

    pub fn p2pkh_prefix(&self) -> [u8; 2] {
        match self {
            ZcashNetwork::Mainnet => [0x1C, 0xB8], // t1 addresses
            ZcashNetwork::Testnet => [0x1D, 0x25], // tm addresses
        }
    }

    pub fn p2sh_prefix(&self) -> [u8; 2] {
        match self {
            ZcashNetwork::Mainnet => [0x1C, 0xBD], // t3 addresses
            ZcashNetwork::Testnet => [0x1C, 0xBA], // t2 addresses
        }
    }

    pub fn to_bitcoin_network(&self) -> bitcoin::Network {
        match self {
            ZcashNetwork::Mainnet => bitcoin::Network::Bitcoin,
            ZcashNetwork::Testnet => bitcoin::Network::Testnet,
        }
    }
}


// ==================== HTLC Parameters ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTLCParams {
    pub recipient_pubkey: String,
    pub refund_pubkey: String,
    pub hash_lock: String,
    pub timelock: u64,
    pub amount: String,
}

// ==================== UTXO Model ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXO {
    pub txid: String,
    pub vout: u32,
    pub amount: String,
    pub script_pubkey: String,
    pub confirmations: u32,
}

// ==================== RPC Models ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZcashRpcRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZcashRpcResponse<T> {
    pub jsonrpc: String,
    pub id: String,
    pub result: Option<T>,
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawTransaction {
    pub txid: String,
    pub version: u32,
    pub locktime: u32,
    pub vin: Vec<TxInput>,
    pub vout: Vec<TxOutput>,
    pub confirmations: Option<u32>,
    pub blockhash: Option<String>,
    pub blocktime: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TxInput {
    pub txid: String,
    pub vout: u32,
    #[serde(rename = "scriptSig")]
    pub script_sig: Option<ScriptSig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScriptSig {
    pub hex: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TxOutput {
    pub value: f64,
    pub n: u32,
    #[serde(rename = "scriptPubKey")]
    pub script_pubkey: ScriptPubKey,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScriptPubKey {
    pub hex: String,
    #[serde(rename = "type")]
    pub script_type: String,
    pub addresses: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ExplorerUTXO {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
    pub script_pubkey: Option<String>,
    pub confirmations: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct HTLCCreationResult {
    pub htlc_id: String,
    pub txid: String,
    pub p2sh_address: String,
    pub redeem_script: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerConfig {
    pub hot_wallet_privkey: String,
    pub hot_wallet_address: String,
    pub max_tx_per_batch: u32,
    pub poll_interval_secs: u64,
    pub max_retry_attempts: u32,
    pub min_confirmations: u32,
    pub network_fee_zec: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerUTXO {
    pub id: String,
    pub txid: String,
    pub vout: u32,
    pub amount: String,
    pub script_pubkey: String,
    pub confirmations: u32,
    pub address: String,
    pub spent: bool,
    pub spent_in_tx: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}


impl From<RelayerUTXO> for UTXO {
    fn from(utxo: RelayerUTXO) -> Self {
        UTXO {
            txid: utxo.txid,
            vout: utxo.vout,
            amount: utxo.amount,
            script_pubkey: utxo.script_pubkey,
            confirmations: utxo.confirmations,
        }
    }
}
