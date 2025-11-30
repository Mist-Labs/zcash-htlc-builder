-- Your SQL goes here
CREATE TABLE relayer_utxos (
    id VARCHAR(255) PRIMARY KEY,
    txid VARCHAR(255) NOT NULL,
    vout INTEGER NOT NULL,
    amount VARCHAR(50) NOT NULL,
    script_pubkey TEXT NOT NULL,
    confirmations INTEGER NOT NULL DEFAULT 0,
    address VARCHAR(255) NOT NULL,
    spent BOOLEAN NOT NULL DEFAULT FALSE,
    spent_in_tx VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(txid, vout)
);

CREATE INDEX idx_relayer_utxos_spent ON relayer_utxos(spent);
CREATE INDEX idx_relayer_utxos_address ON relayer_utxos(address);
CREATE INDEX idx_relayer_utxos_confirmations ON relayer_utxos(confirmations);