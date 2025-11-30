use zcash_htlc_builder::{
    ZcashConfig, ZcashHTLCClient, HTLCParams, UTXO, RelayerUTXO, HTLCState,
    database::Database,
};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, error};

struct AutomatedRelayer {
    client: ZcashHTLCClient,
    database: Arc<Database>,
    hot_wallet_privkey: String,
    hot_wallet_address: String,
    max_tx_per_batch: u32,
    poll_interval: Duration,
    network_fee: String,
}

impl AutomatedRelayer {
    async fn new(config: ZcashConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let relayer_config = config.relayer.clone()
            .ok_or("Relayer config missing in zcash-config.toml")?;
        
        let database = Arc::new(Database::new(
            &config.database_url, 
            config.database_max_connections
        )?);
        
        let client = ZcashHTLCClient::new(config, database.clone());
        
        Ok(Self {
            client,
            database,
            hot_wallet_privkey: relayer_config.hot_wallet_privkey,
            hot_wallet_address: relayer_config.hot_wallet_address,
            max_tx_per_batch: relayer_config.max_tx_per_batch,
            poll_interval: Duration::from_secs(relayer_config.poll_interval_secs),
            network_fee: relayer_config.network_fee_zec,
        })
    }

    async fn process_pending_htlc_creations(&self) -> Result<(), Box<dyn std::error::Error>> {
        let pending = self.database.get_pending_htlcs_for_creation(self.max_tx_per_batch)?;

        for htlc in pending {
            info!("🔨 Processing HTLC creation: {}", htlc.id);

            let funding_utxos = self.get_relayer_utxos().await?;
            
            if funding_utxos.is_empty() {
                error!("❌ No UTXOs available in hot wallet!");
                continue;
            }

            let amount: f64 = htlc.amount.parse().unwrap_or(0.0);
            let fee: f64 = self.network_fee.parse().unwrap_or(0.0001);
            let required = amount + fee;

            let selected_utxos = self.select_utxos(&funding_utxos, required)?;
            
            let params = HTLCParams {
                recipient_pubkey: htlc.recipient_pubkey,
                refund_pubkey: htlc.refund_pubkey,
                hash_lock: htlc.hash_lock,
                timelock: htlc.timelock,
                amount: htlc.amount,
            };

            match self.client.create_htlc(
                params,
                selected_utxos.clone(),
                &self.hot_wallet_address,
                vec![&self.hot_wallet_privkey],
            ).await {
                Ok(result) => {
                    info!("✅ HTLC created: {} with txid: {}", result.htlc_id, result.txid);
                    
                    for utxo in selected_utxos {
                        if let Err(e) = self.database.mark_utxo_spent(
                            &utxo.txid,
                            utxo.vout,
                            &result.txid
                        ) {
                            error!("Failed to mark UTXO spent: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Failed to create HTLC {}: {}", htlc.id, e);
                    let _ = self.database.update_htlc_state(&htlc.id, HTLCState::Failed);
                }
            }
        }

        Ok(())
    }

    async fn process_pending_redemptions(&self) -> Result<(), Box<dyn std::error::Error>> {
        let pending = self.database.get_htlcs_with_signed_redeem_tx(self.max_tx_per_batch)?;

        for htlc in pending {
            if let Some(signed_tx) = htlc.signed_redeem_tx {
                info!("🔓 Broadcasting pre-signed redemption for HTLC: {}", htlc.id);

                match self.client.broadcast_raw_tx(&signed_tx).await {
                    Ok(txid) => {
                        info!("✅ HTLC redeemed: {} with txid: {}", htlc.id, txid);
                        let _ = self.database.update_htlc_state(&htlc.id, HTLCState::Redeemed);
                    }
                    Err(e) => {
                        error!("❌ Failed to broadcast redemption for {}: {}", htlc.id, e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn process_expired_htlcs(&self) -> Result<(), Box<dyn std::error::Error>> {
        let current_block = self.client.get_current_block_height().await?;
        let expired = self.database.get_expired_htlcs(current_block)?;

        for htlc in expired {
            info!("♻️ Processing refund for expired HTLC: {}", htlc.id);

            match self.client.refund_htlc(
                &htlc.id,
                &self.hot_wallet_address,
                &self.hot_wallet_privkey,
            ).await {
                Ok(txid) => {
                    info!("✅ HTLC refunded: {} with txid: {}", htlc.id, txid);
                }
                Err(e) => {
                    error!("❌ Failed to refund HTLC {}: {}", htlc.id, e);
                }
            }
        }

        Ok(())
    }

    async fn get_relayer_utxos(&self) -> Result<Vec<UTXO>, Box<dyn std::error::Error>> {
        let utxos = self.database.get_unspent_relayer_utxos(&self.hot_wallet_address)?;
        Ok(utxos.into_iter().map(Into::into).collect())
    }
    
    fn select_utxos(&self, utxos: &[UTXO], required_amount: f64) -> Result<Vec<UTXO>, Box<dyn std::error::Error>> {
        let mut selected = Vec::new();
        let mut total = 0.0;
        
        for utxo in utxos {
            let amount: f64 = utxo.amount.parse()?;
            selected.push(utxo.clone());
            total += amount;
            
            if total >= required_amount {
                return Ok(selected);
            }
        }
        
        Err("Insufficient UTXOs".into())
    }

    async fn sync_utxos(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔄 Syncing relayer UTXOs...");
        
        let balance = self.database.get_total_relayer_balance(&self.hot_wallet_address)?;
        info!("💰 Current relayer balance: {} ZEC", balance);
        
        Ok(())
    }

    async fn run(&self) {
        info!("🚀 Automated Relayer started");
        info!("💼 Hot wallet: {}", self.hot_wallet_address);
        info!("⏱️  Poll interval: {:?}", self.poll_interval);
        
        let mut ticker = interval(self.poll_interval);
        
        loop {
            ticker.tick().await;
            
            info!("🔄 Processing batch...");
            
            if let Err(e) = self.sync_utxos().await {
                error!("❌ Error syncing UTXOs: {}", e);
            }
            
            if let Err(e) = self.process_pending_htlc_creations().await {
                error!("❌ Error processing HTLC creations: {}", e);
            }
            
            if let Err(e) = self.process_pending_redemptions().await {
                error!("❌ Error processing redemptions: {}", e);
            }
            
            if let Err(e) = self.process_expired_htlcs().await {
                error!("❌ Error processing refunds: {}", e);
            }
            
            info!("✅ Batch complete");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Loading configuration...");
    let config = ZcashConfig::from_default_locations()?;
    
    let relayer = AutomatedRelayer::new(config).await?;
    relayer.run().await;
    
    Ok(())
}