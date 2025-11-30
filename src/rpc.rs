use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tracing::{info, warn};

use crate::{
    ExplorerUTXO, RawTransaction, RpcError, ZcashNetwork, ZcashRpcRequest, ZcashRpcResponse, UTXO,
};

pub struct ZcashRpcClient {
    client: Client,
    rpc_url: String,
    rpc_user: Option<String>,
    rpc_password: Option<String>,
    network: ZcashNetwork,
    explorer_api: String,
}

impl ZcashRpcClient {
    pub fn new(
        rpc_url: String,
        rpc_user: Option<String>,
        rpc_password: Option<String>,
        network: ZcashNetwork,
    ) -> Self {
        let explorer_api = match network {
            ZcashNetwork::Mainnet => "https://api.zcha.in".to_string(),
            ZcashNetwork::Testnet => "https://explorer.testnet.z.cash/api".to_string(),
        };

        Self {
            client: Client::new(),
            rpc_url,
            rpc_user,
            rpc_password,
            network,
            explorer_api,
        }
    }

    pub fn with_custom_explorer(mut self, explorer_url: String) -> Self {
        self.explorer_api = explorer_url;
        self
    }

    async fn call_rpc<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: Vec<Value>,
    ) -> Result<T, RpcClientError> {
        let request = ZcashRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "1".to_string(),
            method: method.to_string(),
            params,
        };

        let mut req_builder = self.client.post(&self.rpc_url).json(&request);

        if let (Some(user), Some(pass)) = (&self.rpc_user, &self.rpc_password) {
            req_builder = req_builder.basic_auth(user, Some(pass));
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| RpcClientError::NetworkError(e.to_string()))?;

        let rpc_response: ZcashRpcResponse<T> = response
            .json()
            .await
            .map_err(|e| RpcClientError::ParseError(e.to_string()))?;

        if let Some(error) = rpc_response.error {
            return Err(RpcClientError::RpcError(error));
        }

        rpc_response.result.ok_or(RpcClientError::NoResult)
    }

    /// Broadcast raw transaction
    pub async fn send_raw_transaction(&self, tx_hex: &str) -> Result<String, RpcClientError> {
        info!("📡 Broadcasting transaction...");

        let txid: String = self
            .call_rpc("sendrawtransaction", vec![serde_json::json!(tx_hex)])
            .await?;

        info!("✅ Transaction broadcast: {}", txid);
        Ok(txid)
    }

    /// Get current block height
    pub async fn get_block_count(&self) -> Result<u64, RpcClientError> {
        let height: u64 = self.call_rpc("getblockcount", vec![]).await?;
        Ok(height)
    }

    /// Get transaction details
    pub async fn get_raw_transaction(&self, txid: &str) -> Result<RawTransaction, RpcClientError> {
        let tx: RawTransaction = self
            .call_rpc(
                "getrawtransaction",
                vec![serde_json::json!(txid), serde_json::json!(true)],
            )
            .await?;
        Ok(tx)
    }

    /// Get transaction confirmations
    pub async fn get_transaction_confirmations(&self, txid: &str) -> Result<u32, RpcClientError> {
        let tx = self.get_raw_transaction(txid).await?;
        Ok(tx.confirmations.unwrap_or(0))
    }

    /// Wait for transaction confirmation
    pub async fn wait_for_confirmations(
        &self,
        txid: &str,
        required_confirmations: u32,
        max_attempts: u32,
    ) -> Result<u32, RpcClientError> {
        info!(
            "⏳ Waiting for {} confirmations on tx: {}",
            required_confirmations, txid
        );

        for attempt in 1..=max_attempts {
            match self.get_transaction_confirmations(txid).await {
                Ok(confirmations) => {
                    if confirmations >= required_confirmations {
                        info!("✅ Transaction confirmed: {} confirmations", confirmations);
                        return Ok(confirmations);
                    }
                    info!(
                        "⏳ Attempt {}/{}: {} confirmations",
                        attempt, max_attempts, confirmations
                    );
                }
                Err(e) => {
                    warn!(
                        "⚠️ Error checking confirmations (attempt {}): {}",
                        attempt, e
                    );
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }

        Err(RpcClientError::ConfirmationTimeout {
            txid: txid.to_string(),
            attempts: max_attempts,
        })
    }

    // ==================== Block Explorer Methods ====================

    /// Query UTXOs for an address using block explorer
    pub async fn get_utxos(&self, address: &str) -> Result<Vec<UTXO>, RpcClientError> {
        info!("🔍 Querying UTXOs for address: {}", address);

        let url = format!("{}/address/{}/utxo", self.explorer_api, address);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RpcClientError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RpcClientError::ExplorerError(format!(
                "HTTP {} from explorer",
                response.status()
            )));
        }

        let utxos: Vec<ExplorerUTXO> = response
            .json()
            .await
            .map_err(|e| RpcClientError::ParseError(e.to_string()))?;

        let converted: Vec<UTXO> = utxos
            .into_iter()
            .map(|u| UTXO {
                txid: u.txid,
                vout: u.vout,
                amount: self.zatoshi_to_zec(u.value),
                script_pubkey: u.script_pubkey.unwrap_or_default(),
                confirmations: u.confirmations.unwrap_or(0),
            })
            .collect();

        info!("✅ Found {} UTXOs", converted.len());
        Ok(converted)
    }

    /// Get address balance
    pub async fn get_balance(&self, address: &str) -> Result<String, RpcClientError> {
        info!("💰 Querying balance for address: {}", address);

        let url = format!("{}/address/{}", self.explorer_api, address);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RpcClientError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RpcClientError::ExplorerError(format!(
                "HTTP {} from explorer",
                response.status()
            )));
        }

        #[derive(Deserialize)]
        struct AddressInfo {
            balance: u64,
        }

        let info: AddressInfo = response
            .json()
            .await
            .map_err(|e| RpcClientError::ParseError(e.to_string()))?;

        let balance_zec = self.zatoshi_to_zec(info.balance);
        info!("✅ Balance: {} ZEC", balance_zec);

        Ok(balance_zec)
    }

    /// Check if transaction is confirmed
    pub async fn is_transaction_confirmed(
        &self,
        txid: &str,
        min_confirmations: u32,
    ) -> Result<bool, RpcClientError> {
        match self.get_transaction_confirmations(txid).await {
            Ok(confirmations) => Ok(confirmations >= min_confirmations),
            Err(_) => Ok(false),
        }
    }

    // ==================== Helper Methods ====================

    fn zatoshi_to_zec(&self, zatoshis: u64) -> String {
        let zec = zatoshis as f64 / 100_000_000.0;
        format!("{:.8}", zec)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RpcClientError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("RPC error: {0}")]
    RpcError(RpcError),

    #[error("No result in RPC response")]
    NoResult,

    #[error("Explorer error: {0}")]
    ExplorerError(String),

    #[error("Confirmation timeout for {txid} after {attempts} attempts")]
    ConfirmationTimeout { txid: String, attempts: u32 },
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Code {}: {}", self.code, self.message)
    }
}
