// @generated automatically by Diesel CLI.

diesel::table! {
    htlc_operations (id) {
        id -> Varchar,
        htlc_id -> Varchar,
        operation_type -> Varchar,
        txid -> Nullable<Varchar>,
        raw_tx_hex -> Nullable<Text>,
        signed_tx_hex -> Nullable<Text>,
        broadcast_at -> Nullable<Timestamptz>,
        confirmed_at -> Nullable<Timestamptz>,
        block_height -> Nullable<Int8>,
        status -> Varchar,
        error_message -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    indexer_checkpoints (id) {
        id -> Int4,
        chain -> Varchar,
        last_block -> Int4,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    zcash_htlcs (id) {
        id -> Varchar,
        txid -> Nullable<Varchar>,
        p2sh_address -> Varchar,
        hash_lock -> Varchar,
        secret -> Nullable<Varchar>,
        timelock -> Int8,
        recipient_pubkey -> Varchar,
        refund_pubkey -> Varchar,
        amount -> Varchar,
        network -> Varchar,
        state -> Int2,
        vout -> Nullable<Int4>,
        script_hex -> Text,
        redeem_script_hex -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(htlc_operations -> zcash_htlcs (htlc_id));

diesel::allow_tables_to_appear_in_same_query!(
    htlc_operations,
    indexer_checkpoints,
    zcash_htlcs,
);
