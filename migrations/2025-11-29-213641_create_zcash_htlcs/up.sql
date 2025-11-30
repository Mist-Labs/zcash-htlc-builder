-- Your SQL goes here
CREATE TABLE zcash_htlcs (
    id VARCHAR PRIMARY KEY,
    txid VARCHAR,
    p2sh_address VARCHAR NOT NULL,
    hash_lock VARCHAR NOT NULL,
    secret VARCHAR,
    timelock BIGINT NOT NULL,
    recipient_pubkey VARCHAR NOT NULL,
    refund_pubkey VARCHAR NOT NULL,
    amount VARCHAR NOT NULL,
    network VARCHAR NOT NULL,
    state SMALLINT NOT NULL DEFAULT 0,
    vout INTEGER,
    script_hex TEXT NOT NULL,
    redeem_script_hex TEXT NOT NULL,
    recipient_address VARCHAR(255),
    signed_redeem_tx TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_zcash_htlcs_txid ON zcash_htlcs(txid);
CREATE INDEX idx_zcash_htlcs_hash_lock ON zcash_htlcs(hash_lock);
CREATE INDEX idx_zcash_htlcs_state ON zcash_htlcs(state);
CREATE INDEX idx_zcash_htlcs_p2sh_address ON zcash_htlcs(p2sh_address);
