-- Your SQL goes here
CREATE TABLE htlc_operations (
    id VARCHAR PRIMARY KEY,
    htlc_id VARCHAR NOT NULL REFERENCES zcash_htlcs(id) ON DELETE CASCADE,
    operation_type VARCHAR NOT NULL,
    txid VARCHAR,
    raw_tx_hex TEXT,
    signed_tx_hex TEXT,
    broadcast_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,
    block_height BIGINT,
    status VARCHAR NOT NULL DEFAULT 'pending',
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_htlc_operations_htlc_id ON htlc_operations(htlc_id);
CREATE INDEX idx_htlc_operations_status ON htlc_operations(status);
CREATE INDEX idx_htlc_operations_txid ON htlc_operations(txid);
