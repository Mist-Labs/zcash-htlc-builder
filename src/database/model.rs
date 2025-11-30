use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::{HTLCOperation, HTLCOperationType, HTLCState, OperationStatus, ZcashHTLC, ZcashNetwork, schema::{htlc_operations, indexer_checkpoints, zcash_htlcs}};

#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = zcash_htlcs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbZcashHTLC {
    pub id: String,
    pub txid: Option<String>,
    pub p2sh_address: String,
    pub hash_lock: String,
    pub secret: Option<String>,
    pub timelock: i64,
    pub recipient_pubkey: String,
    pub refund_pubkey: String,
    pub amount: String,
    pub network: String,
    pub state: i16,
    pub vout: Option<i32>,
    pub script_hex: String,
    pub redeem_script_hex: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = zcash_htlcs)]
pub struct NewZcashHTLC {
    pub id: String,
    pub p2sh_address: String,
    pub hash_lock: String,
    pub timelock: i64,
    pub recipient_pubkey: String,
    pub refund_pubkey: String,
    pub amount: String,
    pub network: String,
    pub state: i16,
    pub script_hex: String,
    pub redeem_script_hex: String,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = htlc_operations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DbHTLCOperation {
    pub id: String,
    pub htlc_id: String,
    pub operation_type: String,
    pub txid: Option<String>,
    pub raw_tx_hex: Option<String>,
    pub signed_tx_hex: Option<String>,
    pub broadcast_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub block_height: Option<i64>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = htlc_operations)]
pub struct NewHTLCOperation {
    pub id: String,
    pub htlc_id: String,
    pub operation_type: String,
    pub raw_tx_hex: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = indexer_checkpoints)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct IndexerCheckpoint {
    pub id: i32,
    pub chain: String,
    pub last_block: i32,
    pub updated_at: DateTime<Utc>,
}


impl From<DbZcashHTLC> for ZcashHTLC {
    fn from(db: DbZcashHTLC) -> Self {
        ZcashHTLC {
            id: db.id,
            txid: db.txid,
            p2sh_address: db.p2sh_address,
            hash_lock: db.hash_lock,
            secret: db.secret,
            timelock: db.timelock as u64,
            recipient_pubkey: db.recipient_pubkey,
            refund_pubkey: db.refund_pubkey,
            amount: db.amount,
            network: ZcashNetwork::from_str(&db.network),
            state: HTLCState::from_i16(db.state),
            vout: db.vout.map(|v| v as u32),
            script_hex: db.script_hex,
            redeem_script_hex: db.redeem_script_hex,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}

impl From<DbHTLCOperation> for HTLCOperation {
    fn from(db: DbHTLCOperation) -> Self {
        HTLCOperation {
            id: db.id,
            htlc_id: db.htlc_id,
            operation_type: HTLCOperationType::from_str(&db.operation_type),
            txid: db.txid,
            raw_tx_hex: db.raw_tx_hex,
            signed_tx_hex: db.signed_tx_hex,
            broadcast_at: db.broadcast_at,
            confirmed_at: db.confirmed_at,
            block_height: db.block_height.map(|b| b as u64),
            status: OperationStatus::from_str(&db.status),
            error_message: db.error_message,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}
