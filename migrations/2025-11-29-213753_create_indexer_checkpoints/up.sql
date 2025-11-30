-- Your SQL goes here
CREATE TABLE indexer_checkpoints (
    id SERIAL PRIMARY KEY,
    chain VARCHAR NOT NULL UNIQUE,
    last_block INTEGER NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_indexer_checkpoints_chain ON indexer_checkpoints(chain);
