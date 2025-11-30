use bitcoin::blockdata::opcodes::{self, OP_FALSE, OP_TRUE};
use bitcoin::blockdata::script::{Builder, Script};
use bitcoin::hashes::{hash160, Hash};
use ripemd::Digest;
use sha2::Sha256;

use crate::{HTLCParams, ZcashNetwork};

/// Build P2SH HTLC script according to ZIP-300
///
/// Script format:
/// OP_IF
///     OP_SHA256 <hash_lock> OP_EQUALVERIFY
///     <recipient_pubkey> OP_CHECKSIG
/// OP_ELSE
///     <timelock> OP_CHECKLOCKTIMEVERIFY OP_DROP
///     <refund_pubkey> OP_CHECKSIG
/// OP_ENDIF
#[derive(Clone)]
pub struct HTLCScriptBuilder {
    network: ZcashNetwork,
}

impl HTLCScriptBuilder {
    pub fn new(network: ZcashNetwork) -> Self {
        Self { network }
    }

    pub fn build_htlc_script(&self, params: &HTLCParams) -> Result<Script, HTLCScriptError> {
        let hash_lock_bytes =
            hex::decode(&params.hash_lock).map_err(|_| HTLCScriptError::InvalidHashLock)?;

        if hash_lock_bytes.len() != 32 {
            return Err(HTLCScriptError::InvalidHashLockLength);
        }

        let recipient_pubkey =
            hex::decode(&params.recipient_pubkey).map_err(|_| HTLCScriptError::InvalidPublicKey)?;

        let refund_pubkey =
            hex::decode(&params.refund_pubkey).map_err(|_| HTLCScriptError::InvalidPublicKey)?;

        let script = Builder::new()
            .push_opcode(opcodes::all::OP_IF)
            .push_opcode(opcodes::all::OP_SHA256)
            .push_slice(&hash_lock_bytes)
            .push_opcode(opcodes::all::OP_EQUALVERIFY)
            .push_slice(&recipient_pubkey)
            .push_opcode(opcodes::all::OP_CHECKSIG)
            .push_opcode(opcodes::all::OP_ELSE)
            .push_int(params.timelock as i64)
            .push_opcode(opcodes::all::OP_CLTV)
            .push_opcode(opcodes::all::OP_DROP)
            .push_slice(&refund_pubkey)
            .push_opcode(opcodes::all::OP_CHECKSIG)
            .push_opcode(opcodes::all::OP_ENDIF)
            .into_script();

        Ok(script)
    }

    pub fn script_to_p2sh_address(&self, script: &Script) -> Result<String, HTLCScriptError> {
        let script_hash = hash160::Hash::hash(script.as_bytes());
        let prefix = self.network.p2sh_prefix();

        let mut address_bytes = Vec::new();
        address_bytes.extend_from_slice(&prefix);
        address_bytes.extend_from_slice(script_hash.as_ref());

        let checksum = self.double_sha256_checksum(&address_bytes);
        address_bytes.extend_from_slice(&checksum[..4]);

        Ok(bs58::encode(address_bytes).into_string())
    }

    pub fn build_redeem_input(
        &self,
        secret: &str,
        signature: &[u8],
    ) -> Result<Script, HTLCScriptError> {
        let secret_bytes = hex::decode(secret).map_err(|_| HTLCScriptError::InvalidSecret)?;

        let script = Builder::new()
            .push_slice(signature)
            .push_slice(&secret_bytes)
            .push_opcode(OP_TRUE)
            .into_script();

        Ok(script)
    }

    pub fn build_refund_input(&self, signature: &[u8]) -> Script {
        Builder::new()
            .push_slice(signature)
            .push_opcode(OP_FALSE)
            .into_script()
    }

    pub fn verify_secret(&self, secret: &str, hash_lock: &str) -> bool {
        let secret_bytes = match hex::decode(secret) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        let mut hasher = Sha256::new();
        hasher.update(&secret_bytes);
        let computed_hash = hex::encode(hasher.finalize());

        computed_hash == hash_lock
    }

    fn double_sha256_checksum(&self, data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let first_hash = hasher.finalize();

        let mut hasher = Sha256::new();
        hasher.update(first_hash);
        hasher.finalize().to_vec()
    }

    pub fn p2sh_script_pubkey(&self, script: &Script) -> Script {
        let script_hash = hash160::Hash::hash(script.as_bytes());

        Builder::new()
            .push_opcode(opcodes::all::OP_HASH160)
            .push_slice(script_hash.as_ref())
            .push_opcode(opcodes::all::OP_EQUAL)
            .into_script()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HTLCScriptError {
    #[error("Invalid hash lock format")]
    InvalidHashLock,

    #[error("Invalid hash lock length (expected 32 bytes)")]
    InvalidHashLockLength,

    #[error("Invalid public key format")]
    InvalidPublicKey,

    #[error("Invalid secret format")]
    InvalidSecret,

    #[error("Script building failed: {0}")]
    BuildError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_htlc_script() {
        let builder = HTLCScriptBuilder::new(ZcashNetwork::Testnet);

        let params = HTLCParams {
            recipient_pubkey: format!("02{}", "a".repeat(64)),
            refund_pubkey: format!("03{}", "b".repeat(64)),
            hash_lock: "a".repeat(64),
            timelock: 100,
            amount: "1.0".to_string(),
        };

        let script = builder.build_htlc_script(&params).unwrap();
        assert!(!script.as_bytes().is_empty());
    }

    #[test]
    fn test_verify_secret() {
        let builder = HTLCScriptBuilder::new(ZcashNetwork::Testnet);

        let secret = "deadbeef";
        let mut hasher = Sha256::new();
        hasher.update(hex::decode(secret).unwrap());
        let hash_lock = hex::encode(hasher.finalize());

        assert!(builder.verify_secret(secret, &hash_lock));
        assert!(!builder.verify_secret("badbeef", &hash_lock));
    }
}
