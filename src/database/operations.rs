use chrono::Utc;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use tracing::info;

use crate::{HTLCOperation, HTLCState, OperationStatus, RelayerUTXO, ZcashHTLC, ZcashNetwork};
use crate::database::model::{DbHTLCOperation, DbRelayerUTXO, DbZcashHTLC, NewHTLCOperation, NewRelayerUTXO, NewZcashHTLC};

use super::connections::{Database, DatabaseError};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

impl Database {
    pub fn create_htlc(&self, htlc: &ZcashHTLC) -> Result<(), DatabaseError> {
        use crate::models::schema::zcash_htlcs;

        let mut conn = self.get_connection()?;

        let new_htlc = NewZcashHTLC {
            id: htlc.id.clone(),
            p2sh_address: htlc.p2sh_address.clone(),
            hash_lock: htlc.hash_lock.clone(),
            timelock: htlc.timelock as i64,
            recipient_pubkey: htlc.recipient_pubkey.clone(),
            refund_pubkey: htlc.refund_pubkey.clone(),
            amount: htlc.amount.clone(),
            network: htlc.network.as_str().to_string(),
            state: htlc.state as i16,
            script_hex: htlc.script_hex.clone(),
            redeem_script_hex: htlc.redeem_script_hex.clone(),
        };

        diesel::insert_into(zcash_htlcs::table)
            .values(&new_htlc)
            .execute(&mut conn)?;

        info!("📝 Created HTLC record: {}", htlc.id);
        Ok(())
    }

    pub fn get_htlc_by_id(&self, htlc_id: &str) -> Result<ZcashHTLC, DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;

        let mut conn = self.get_connection()?;

        let htlc = dsl::zcash_htlcs
            .filter(dsl::id.eq(htlc_id))
            .select(DbZcashHTLC::as_select())
            .first::<DbZcashHTLC>(&mut conn)
            .map_err(|_| DatabaseError::HTLCNotFound(htlc_id.to_string()))?;

        Ok(htlc.into())
    }

    pub fn get_htlc_by_txid(&self, txid: &str) -> Result<ZcashHTLC, DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;

        let mut conn = self.get_connection()?;

        let htlc = dsl::zcash_htlcs
            .filter(dsl::txid.eq(txid))
            .select(DbZcashHTLC::as_select())
            .first::<DbZcashHTLC>(&mut conn)
            .map_err(|_| DatabaseError::HTLCNotFound(txid.to_string()))?;

        Ok(htlc.into())
    }

    pub fn get_htlc_by_hash_lock(
        &self,
        hash_lock: &str,
    ) -> Result<Option<ZcashHTLC>, DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;

        let mut conn = self.get_connection()?;

        let htlc = dsl::zcash_htlcs
            .filter(dsl::hash_lock.eq(hash_lock))
            .select(DbZcashHTLC::as_select())
            .first::<DbZcashHTLC>(&mut conn)
            .optional()?;

        Ok(htlc.map(Into::into))
    }

    pub fn update_htlc_txid(
        &self,
        htlc_id: &str,
        txid: &str,
        vout: u32,
    ) -> Result<(), DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;

        let mut conn = self.get_connection()?;

        diesel::update(dsl::zcash_htlcs.filter(dsl::id.eq(htlc_id)))
            .set((
                dsl::txid.eq(txid),
                dsl::vout.eq(vout as i32),
                dsl::state.eq(HTLCState::Locked as i16),
                dsl::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)?;

        info!("🔄 Updated HTLC {} with txid: {}", htlc_id, txid);
        Ok(())
    }

    pub fn update_htlc_state(&self, htlc_id: &str, state: HTLCState) -> Result<(), DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;

        let mut conn = self.get_connection()?;

        diesel::update(dsl::zcash_htlcs.filter(dsl::id.eq(htlc_id)))
            .set((dsl::state.eq(state as i16), dsl::updated_at.eq(Utc::now())))
            .execute(&mut conn)?;

        info!("🔄 Updated HTLC {} state to: {:?}", htlc_id, state);
        Ok(())
    }

    pub fn update_htlc_secret(&self, htlc_id: &str, secret: &str) -> Result<(), DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;

        let mut conn = self.get_connection()?;

        diesel::update(dsl::zcash_htlcs.filter(dsl::id.eq(htlc_id)))
            .set((dsl::secret.eq(secret), dsl::updated_at.eq(Utc::now())))
            .execute(&mut conn)?;

        info!("🔐 Updated HTLC {} with secret", htlc_id);
        Ok(())
    }

    pub fn get_pending_htlcs(
        &self,
        network: ZcashNetwork,
    ) -> Result<Vec<ZcashHTLC>, DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;

        let mut conn = self.get_connection()?;

        let htlcs = dsl::zcash_htlcs
            .filter(dsl::network.eq(network.as_str()))
            .filter(dsl::state.eq(HTLCState::Locked as i16))
            .select(DbZcashHTLC::as_select())
            .load::<DbZcashHTLC>(&mut conn)?;

        Ok(htlcs.into_iter().map(Into::into).collect())
    }

    pub fn get_expired_htlcs(&self, current_block: u64) -> Result<Vec<ZcashHTLC>, DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;

        let mut conn = self.get_connection()?;

        let htlcs = dsl::zcash_htlcs
            .filter(dsl::state.eq(HTLCState::Locked as i16))
            .filter(dsl::timelock.lt(current_block as i64))
            .select(DbZcashHTLC::as_select())
            .load::<DbZcashHTLC>(&mut conn)?;

        Ok(htlcs.into_iter().map(Into::into).collect())
    }

    pub fn create_operation(&self, operation: &HTLCOperation) -> Result<(), DatabaseError> {
        use crate::models::schema::htlc_operations;

        let mut conn = self.get_connection()?;

        let new_op = NewHTLCOperation {
            id: operation.id.clone(),
            htlc_id: operation.htlc_id.clone(),
            operation_type: operation.operation_type.as_str().to_string(),
            raw_tx_hex: operation.raw_tx_hex.clone(),
            status: operation.status.as_str().to_string(),
        };

        diesel::insert_into(htlc_operations::table)
            .values(&new_op)
            .execute(&mut conn)?;

        info!("📝 Created operation record: {}", operation.id);
        Ok(())
    }

    pub fn update_operation_signed(
        &self,
        operation_id: &str,
        signed_tx_hex: &str,
    ) -> Result<(), DatabaseError> {
        use crate::models::schema::htlc_operations::dsl;

        let mut conn = self.get_connection()?;

        diesel::update(dsl::htlc_operations.filter(dsl::id.eq(operation_id)))
            .set((
                dsl::signed_tx_hex.eq(signed_tx_hex),
                dsl::status.eq(OperationStatus::Signed.as_str()),
                dsl::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)?;

        info!("✍️ Signed operation: {}", operation_id);
        Ok(())
    }

    pub fn update_operation_broadcast(
        &self,
        operation_id: &str,
        txid: &str,
    ) -> Result<(), DatabaseError> {
        use crate::models::schema::htlc_operations::dsl;

        let mut conn = self.get_connection()?;

        diesel::update(dsl::htlc_operations.filter(dsl::id.eq(operation_id)))
            .set((
                dsl::txid.eq(txid),
                dsl::status.eq(OperationStatus::Broadcast.as_str()),
                dsl::broadcast_at.eq(Utc::now()),
                dsl::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)?;

        info!("📡 Broadcast operation: {}", operation_id);
        Ok(())
    }

    pub fn update_operation_confirmed(
        &self,
        operation_id: &str,
        block_height: u64,
    ) -> Result<(), DatabaseError> {
        use crate::models::schema::htlc_operations::dsl;

        let mut conn = self.get_connection()?;

        diesel::update(dsl::htlc_operations.filter(dsl::id.eq(operation_id)))
            .set((
                dsl::status.eq(OperationStatus::Confirmed.as_str()),
                dsl::block_height.eq(block_height as i64),
                dsl::confirmed_at.eq(Utc::now()),
                dsl::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)?;

        info!("✅ Confirmed operation: {}", operation_id);
        Ok(())
    }

    pub fn update_operation_failed(
        &self,
        operation_id: &str,
        error: &str,
    ) -> Result<(), DatabaseError> {
        use crate::models::schema::htlc_operations::dsl;

        let mut conn = self.get_connection()?;

        diesel::update(dsl::htlc_operations.filter(dsl::id.eq(operation_id)))
            .set((
                dsl::status.eq(OperationStatus::Failed.as_str()),
                dsl::error_message.eq(error),
                dsl::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)?;

        info!("❌ Failed operation: {} - {}", operation_id, error);
        Ok(())
    }

    pub fn get_operation_by_id(&self, operation_id: &str) -> Result<HTLCOperation, DatabaseError> {
        use crate::models::schema::htlc_operations::dsl;

        let mut conn = self.get_connection()?;

        let operation = dsl::htlc_operations
            .filter(dsl::id.eq(operation_id))
            .select(DbHTLCOperation::as_select())
            .first::<DbHTLCOperation>(&mut conn)
            .map_err(|_| DatabaseError::OperationNotFound(operation_id.to_string()))?;

        Ok(operation.into())
    }

    pub fn get_operations_by_htlc(
        &self,
        htlc_id: &str,
    ) -> Result<Vec<HTLCOperation>, DatabaseError> {
        use crate::models::schema::htlc_operations::dsl;

        let mut conn = self.get_connection()?;

        let operations = dsl::htlc_operations
            .filter(dsl::htlc_id.eq(htlc_id))
            .order(dsl::created_at.desc())
            .select(DbHTLCOperation::as_select())
            .load::<DbHTLCOperation>(&mut conn)?;

        Ok(operations.into_iter().map(Into::into).collect())
    }

    pub fn save_checkpoint(&self, chain: &str, block_height: u32) -> Result<(), DatabaseError> {
        use crate::models::schema::indexer_checkpoints::dsl;

        let mut conn = self.get_connection()?;

        diesel::insert_into(dsl::indexer_checkpoints)
            .values((
                dsl::chain.eq(chain),
                dsl::last_block.eq(block_height as i32),
                dsl::updated_at.eq(Utc::now()),
            ))
            .on_conflict(dsl::chain)
            .do_update()
            .set((
                dsl::last_block.eq(block_height as i32),
                dsl::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)?;

        Ok(())
    }

    pub fn get_checkpoint(&self, chain: &str) -> Result<Option<u32>, DatabaseError> {
        use crate::models::schema::indexer_checkpoints::dsl;

        let mut conn = self.get_connection()?;

        let result = dsl::indexer_checkpoints
            .filter(dsl::chain.eq(chain))
            .select(dsl::last_block)
            .first::<i32>(&mut conn)
            .optional()?;

        Ok(result.map(|b| b as u32))
    }

    pub fn create_relayer_utxo(&self, utxo: &RelayerUTXO) -> Result<(), DatabaseError> {
        use crate::models::schema::relayer_utxos;
        
        let mut conn = self.get_connection()?;
        
        let new_utxo = NewRelayerUTXO {
            id: utxo.id.clone(),
            txid: utxo.txid.clone(),
            vout: utxo.vout as i32,
            amount: utxo.amount.clone(),
            script_pubkey: utxo.script_pubkey.clone(),
            confirmations: utxo.confirmations as i32,
            address: utxo.address.clone(),
        };
        
        diesel::insert_into(relayer_utxos::table)
            .values(&new_utxo)
            .on_conflict((relayer_utxos::txid, relayer_utxos::vout))
            .do_nothing()
            .execute(&mut conn)?;
        
        info!("📦 Created relayer UTXO: {}:{}", utxo.txid, utxo.vout);
        Ok(())
    }
    
    pub fn get_unspent_relayer_utxos(&self, address: &str) -> Result<Vec<RelayerUTXO>, DatabaseError> {
        use crate::models::schema::relayer_utxos::dsl;
        
        let mut conn = self.get_connection()?;
        
        let utxos = dsl::relayer_utxos
            .filter(dsl::address.eq(address))
            .filter(dsl::spent.eq(false))
            .filter(dsl::confirmations.ge(1))
            .order(dsl::amount.desc())
            .select(DbRelayerUTXO::as_select())
            .load::<DbRelayerUTXO>(&mut conn)?;
        
        Ok(utxos.into_iter().map(Into::into).collect())
    }
    
    pub fn mark_utxo_spent(&self, txid: &str, vout: u32, spent_in_tx: &str) -> Result<(), DatabaseError> {
        use crate::models::schema::relayer_utxos::dsl;
        
        let mut conn = self.get_connection()?;
        
        diesel::update(
            dsl::relayer_utxos
                .filter(dsl::txid.eq(txid))
                .filter(dsl::vout.eq(vout as i32))
        )
        .set((
            dsl::spent.eq(true),
            dsl::spent_in_tx.eq(spent_in_tx),
            dsl::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)?;
        
        info!("✅ Marked UTXO spent: {}:{} in tx {}", txid, vout, spent_in_tx);
        Ok(())
    }
    
    pub fn update_utxo_confirmations(&self, txid: &str, vout: u32, confirmations: u32) -> Result<(), DatabaseError> {
        use crate::models::schema::relayer_utxos::dsl;
        
        let mut conn = self.get_connection()?;
        
        diesel::update(
            dsl::relayer_utxos
                .filter(dsl::txid.eq(txid))
                .filter(dsl::vout.eq(vout as i32))
        )
        .set((
            dsl::confirmations.eq(confirmations as i32),
            dsl::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)?;
        
        Ok(())
    }
    
    pub fn get_total_relayer_balance(&self, address: &str) -> Result<f64, DatabaseError> {
        use crate::models::schema::relayer_utxos::dsl;
        use diesel::dsl::sum;
        
        let mut conn = self.get_connection()?;
        
        let utxos: Vec<String> = dsl::relayer_utxos
            .filter(dsl::address.eq(address))
            .filter(dsl::spent.eq(false))
            .select(dsl::amount)
            .load(&mut conn)?;
        
        let total: f64 = utxos.iter()
            .filter_map(|s| s.parse::<f64>().ok())
            .sum();
        
        Ok(total)
    }

    pub fn get_pending_htlcs_for_creation(&self, limit: u32) -> Result<Vec<ZcashHTLC>, DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;
        
        let mut conn = self.get_connection()?;
        
        let htlcs = dsl::zcash_htlcs
            .filter(dsl::state.eq(HTLCState::Pending as i16))
            .filter(dsl::txid.is_null())
            .order(dsl::created_at.asc())
            .limit(limit as i64)
            .select(DbZcashHTLC::as_select())
            .load::<DbZcashHTLC>(&mut conn)?;
        
        Ok(htlcs.into_iter().map(Into::into).collect())
    }
    
    pub fn get_htlcs_with_signed_redeem_tx(&self, limit: u32) -> Result<Vec<ZcashHTLC>, DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;
        
        let mut conn = self.get_connection()?;
        
        let htlcs = dsl::zcash_htlcs
            .filter(dsl::state.eq(HTLCState::Locked as i16))
            .filter(dsl::signed_redeem_tx.is_not_null())
            .order(dsl::created_at.asc())
            .limit(limit as i64)
            .select(DbZcashHTLC::as_select())
            .load::<DbZcashHTLC>(&mut conn)?;
        
        Ok(htlcs.into_iter().map(Into::into).collect())
    }
    
    // ==================== HTLC Recipient Operations ====================
    
    pub fn update_htlc_recipient(&self, htlc_id: &str, recipient_address: &str) -> Result<(), DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;
        
        let mut conn = self.get_connection()?;
        
        diesel::update(dsl::zcash_htlcs.filter(dsl::id.eq(htlc_id)))
            .set((
                dsl::recipient_address.eq(recipient_address),
                dsl::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)?;
        
        Ok(())
    }
    
    pub fn store_signed_redeem_tx(&self, htlc_id: &str, signed_tx: &str) -> Result<(), DatabaseError> {
        use crate::models::schema::zcash_htlcs::dsl;
        
        let mut conn = self.get_connection()?;
        
        diesel::update(dsl::zcash_htlcs.filter(dsl::id.eq(htlc_id)))
            .set((
                dsl::signed_redeem_tx.eq(signed_tx),
                dsl::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)?;
        
        info!("✍️ Stored signed redeem tx for HTLC: {}", htlc_id);
        Ok(())
    }
}
