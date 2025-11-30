use bitcoin::blockdata::script::Script;
use bitcoin::blockdata::transaction::Transaction;
use bitcoin::EcdsaSighashType;
use secp256k1::{ecdsa::Signature, Message, PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};

use crate::HTLCScriptBuilder;

pub struct TransactionSigner {
    secp: Secp256k1<secp256k1::All>,
    script_builder: HTLCScriptBuilder,
}

impl TransactionSigner {
    pub fn new(script_builder: HTLCScriptBuilder) -> Self {
        Self {
            secp: Secp256k1::new(),
            script_builder,
        }
    }

    pub fn sign_htlc_creation(
        &self,
        mut tx: Transaction,
        input_scripts: Vec<Script>,
        private_keys: Vec<&str>,
    ) -> Result<Transaction, SignerError> {
        if tx.input.len() != input_scripts.len() || tx.input.len() != private_keys.len() {
            return Err(SignerError::MismatchedInputs);
        }

        for (i, (script_pubkey, privkey_hex)) in
            input_scripts.iter().zip(private_keys.iter()).enumerate()
        {
            let privkey = self.parse_privkey(privkey_hex)?;
            let signature = self.sign_input(&tx, i, script_pubkey, &privkey)?;

            let pubkey = PublicKey::from_secret_key(&self.secp, &privkey);
            let script_sig = bitcoin::blockdata::script::Builder::new()
                .push_slice(&signature)
                .push_slice(&pubkey.serialize())
                .into_script();

            tx.input[i].script_sig = script_sig;
        }

        Ok(tx)
    }

    pub fn sign_htlc_redeem(
        &self,
        mut tx: Transaction,
        input_index: usize,
        redeem_script: &Script,
        secret: &str,
        privkey_hex: &str,
    ) -> Result<Transaction, SignerError> {
        let privkey = self.parse_privkey(privkey_hex)?;
        let signature = self.sign_input(&tx, input_index, redeem_script, &privkey)?;

        let script_sig = self
            .script_builder
            .build_redeem_input(secret, &signature)
            .map_err(|e| SignerError::ScriptError(e.to_string()))?;

        let final_script_sig = bitcoin::blockdata::script::Builder::new()
            .push_slice(script_sig.as_bytes())
            .push_slice(redeem_script.as_bytes())
            .into_script();

        tx.input[input_index].script_sig = final_script_sig;

        Ok(tx)
    }

    pub fn sign_htlc_refund(
        &self,
        mut tx: Transaction,
        input_index: usize,
        redeem_script: &Script,
        privkey_hex: &str,
    ) -> Result<Transaction, SignerError> {
        let privkey = self.parse_privkey(privkey_hex)?;
        let signature = self.sign_input(&tx, input_index, redeem_script, &privkey)?;

        let script_sig = self.script_builder.build_refund_input(&signature);

        let final_script_sig = bitcoin::blockdata::script::Builder::new()
            .push_slice(script_sig.as_bytes())
            .push_slice(redeem_script.as_bytes())
            .into_script();

        tx.input[input_index].script_sig = final_script_sig;

        Ok(tx)
    }

    fn sign_input(
        &self,
        tx: &Transaction,
        input_index: usize,
        script_pubkey: &Script,
        privkey: &SecretKey,
    ) -> Result<Vec<u8>, SignerError> {
        let sighash = tx.signature_hash(input_index, script_pubkey, EcdsaSighashType::All.to_u32());

        let message = Message::from_digest_slice(&sighash[..])
            .map_err(|e| SignerError::MessageError(e.to_string()))?;

        let signature = self.secp.sign_ecdsa(&message, privkey);

        let mut sig_bytes = signature.serialize_der().to_vec();
        sig_bytes.push(EcdsaSighashType::All.to_u32() as u8);

        Ok(sig_bytes)
    }

    fn parse_privkey(&self, hex: &str) -> Result<SecretKey, SignerError> {
        let bytes = hex::decode(hex).map_err(|_| SignerError::InvalidPrivateKey)?;

        SecretKey::from_slice(&bytes).map_err(|_| SignerError::InvalidPrivateKey)
    }

    pub fn generate_privkey(&self) -> String {
        let (secret_key, _) = self.secp.generate_keypair(&mut rand::thread_rng());
        hex::encode(secret_key.secret_bytes())
    }

    pub fn derive_pubkey(&self, privkey_hex: &str) -> Result<String, SignerError> {
        let privkey = self.parse_privkey(privkey_hex)?;
        let pubkey = PublicKey::from_secret_key(&self.secp, &privkey);
        Ok(hex::encode(pubkey.serialize()))
    }

    pub fn verify_signature(
        &self,
        message: &[u8],
        signature: &str,
        pubkey_hex: &str,
    ) -> Result<bool, SignerError> {
        let sig_bytes = hex::decode(signature).map_err(|_| SignerError::InvalidSignature)?;

        let signature = Signature::from_der(&sig_bytes[..sig_bytes.len() - 1])
            .map_err(|_| SignerError::InvalidSignature)?;

        let pubkey_bytes = hex::decode(pubkey_hex).map_err(|_| SignerError::InvalidPublicKey)?;

        let pubkey =
            PublicKey::from_slice(&pubkey_bytes).map_err(|_| SignerError::InvalidPublicKey)?;

        let msg = Message::from_digest_slice(message)
            .map_err(|e| SignerError::MessageError(e.to_string()))?;

        Ok(self.secp.verify_ecdsa(&msg, &signature, &pubkey).is_ok())
    }

    pub fn generate_hash_lock(&self, secret: &str) -> String {
        let secret_bytes = hex::decode(secret).unwrap_or_else(|_| secret.as_bytes().to_vec());
        let mut hasher = Sha256::new();
        hasher.update(&secret_bytes);
        hex::encode(hasher.finalize())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignerError {
    #[error("Invalid private key format")]
    InvalidPrivateKey,

    #[error("Invalid public key format")]
    InvalidPublicKey,

    #[error("Invalid signature format")]
    InvalidSignature,

    #[error("Mismatched number of inputs and keys")]
    MismatchedInputs,

    #[error("Sighash error: {0}")]
    SighashError(String),

    #[error("Message error: {0}")]
    MessageError(String),

    #[error("Script error: {0}")]
    ScriptError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ZcashNetwork;

    #[test]
    fn test_generate_privkey() {
        let script_builder = HTLCScriptBuilder::new(ZcashNetwork::Testnet);
        let signer = TransactionSigner::new(script_builder);

        let privkey = signer.generate_privkey();
        assert_eq!(privkey.len(), 64);
    }

    #[test]
    fn test_derive_pubkey() {
        let script_builder = HTLCScriptBuilder::new(ZcashNetwork::Testnet);
        let signer = TransactionSigner::new(script_builder);

        let privkey = signer.generate_privkey();
        let pubkey = signer.derive_pubkey(&privkey).unwrap();
        assert!(pubkey.len() == 66 || pubkey.len() == 130);
    }

    #[test]
    fn test_generate_hash_lock() {
        let script_builder = HTLCScriptBuilder::new(ZcashNetwork::Testnet);
        let signer = TransactionSigner::new(script_builder);

        let secret = "deadbeef";
        let hash_lock = signer.generate_hash_lock(secret);
        assert_eq!(hash_lock.len(), 64);
    }
}
