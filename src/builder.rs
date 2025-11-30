use bitcoin::blockdata::script::Script;
use bitcoin::blockdata::transaction::{OutPoint, Transaction, TxIn, TxOut};
use bitcoin::consensus::encode;
use bitcoin::hash_types::Txid;
use bitcoin::{PackedLockTime, Sequence, Witness};
use std::str::FromStr;

use crate::models::{HTLCParams, UTXO, ZcashNetwork};
use crate::script::HTLCScriptBuilder;

const DUST_THRESHOLD: u64 = 546;
const DEFAULT_FEE_RATE: u64 = 1000;

pub struct TransactionBuilder {
    network: ZcashNetwork,
    script_builder: HTLCScriptBuilder,
}

impl TransactionBuilder {
    pub fn new(network: ZcashNetwork) -> Self {
        Self {
            network,
            script_builder: HTLCScriptBuilder::new(network),
        }
    }

    pub fn build_htlc_tx(
        &self,
        params: &HTLCParams,
        utxos: Vec<UTXO>,
        change_address: &str,
    ) -> Result<(Transaction, Script), TxBuilderError> {
        let amount_sat = self.parse_amount(&params.amount)?;
        
        if amount_sat < DUST_THRESHOLD {
            return Err(TxBuilderError::AmountTooSmall);
        }

        let redeem_script = self.script_builder.build_htlc_script(params)
            .map_err(|e| TxBuilderError::ScriptError(e.to_string()))?;

        let script_pubkey = self.script_builder.p2sh_script_pubkey(&redeem_script);

        let inputs: Vec<TxIn> = utxos
            .iter()
            .map(|utxo| {
                let txid = Txid::from_str(&utxo.txid)
                    .map_err(|_| TxBuilderError::InvalidTxid)?;
                
                Ok(TxIn {
                    previous_output: OutPoint {
                        txid,
                        vout: utxo.vout,
                    },
                    script_sig: Script::new(),
                    sequence: Sequence(0xFFFFFFFF),
                    witness: Witness::default(),
                })
            })
            .collect::<Result<Vec<_>, TxBuilderError>>()?;

        let total_input: u64 = utxos
            .iter()
            .map(|utxo| self.parse_amount(&utxo.amount))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .sum();

        let estimated_size = self.estimate_tx_size(inputs.len(), 2);
        let fee = (estimated_size as u64 * DEFAULT_FEE_RATE) / 1000;

        if total_input < amount_sat + fee {
            return Err(TxBuilderError::InsufficientFunds {
                required: amount_sat + fee,
                available: total_input,
            });
        }

        let mut outputs = vec![TxOut {
            value: amount_sat,
            script_pubkey,
        }];

        let change = total_input - amount_sat - fee;
        if change > DUST_THRESHOLD {
            let change_script = self.address_to_script_pubkey(change_address)?;
            outputs.push(TxOut {
                value: change,
                script_pubkey: change_script,
            });
        }

        let tx = Transaction {
            version: 4,
            lock_time: PackedLockTime(0),
            input: inputs,
            output: outputs,
        };

        Ok((tx, redeem_script))
    }

    pub fn build_redeem_tx(
        &self,
        htlc_txid: &str,
        htlc_vout: u32,
        htlc_amount: &str,
        secret: &str,
        redeem_script: &Script,
        recipient_address: &str,
    ) -> Result<Transaction, TxBuilderError> {
        let txid = Txid::from_str(htlc_txid)
            .map_err(|_| TxBuilderError::InvalidTxid)?;

        let amount_sat = self.parse_amount(htlc_amount)?;
        let estimated_size = self.estimate_tx_size(1, 1);
        let fee = (estimated_size as u64 * DEFAULT_FEE_RATE) / 1000;

        if amount_sat <= fee {
            return Err(TxBuilderError::AmountTooSmall);
        }

        let input = TxIn {
            previous_output: OutPoint {
                txid,
                vout: htlc_vout,
            },
            script_sig: Script::new(),
            sequence: Sequence(0xFFFFFFFF),
            witness: Witness::default(),
        };

        let output_script = self.address_to_script_pubkey(recipient_address)?;
        let output = TxOut {
            value: amount_sat - fee,
            script_pubkey: output_script,
        };

        let tx = Transaction {
            version: 4,
            lock_time: PackedLockTime(0),
            input: vec![input],
            output: vec![output],
        };

        Ok(tx)
    }

    pub fn build_refund_tx(
        &self,
        htlc_txid: &str,
        htlc_vout: u32,
        htlc_amount: &str,
        timelock: u64,
        redeem_script: &Script,
        refund_address: &str,
    ) -> Result<Transaction, TxBuilderError> {
        let txid = Txid::from_str(htlc_txid)
            .map_err(|_| TxBuilderError::InvalidTxid)?;

        let amount_sat = self.parse_amount(htlc_amount)?;
        let estimated_size = self.estimate_tx_size(1, 1);
        let fee = (estimated_size as u64 * DEFAULT_FEE_RATE) / 1000;

        if amount_sat <= fee {
            return Err(TxBuilderError::AmountTooSmall);
        }

        let input = TxIn {
            previous_output: OutPoint {
                txid,
                vout: htlc_vout,
            },
            script_sig: Script::new(),
            sequence: Sequence(0xFFFFFFFF),
            witness: Witness::default(),
        };

        let output_script = self.address_to_script_pubkey(refund_address)?;
        let output = TxOut {
            value: amount_sat - fee,
            script_pubkey: output_script,
        };

        let tx = Transaction {
            version: 4,
            lock_time: PackedLockTime(timelock as u32),
            input: vec![input],
            output: vec![output],
        };

        Ok(tx)
    }

    pub fn serialize_tx(&self, tx: &Transaction) -> String {
        hex::encode(encode::serialize(tx))
    }

    pub fn deserialize_tx(&self, hex: &str) -> Result<Transaction, TxBuilderError> {
        let bytes = hex::decode(hex)
            .map_err(|_| TxBuilderError::InvalidHex)?;
        
        encode::deserialize(&bytes)
            .map_err(|e| TxBuilderError::DeserializationError(e.to_string()))
    }

    fn parse_amount(&self, amount_str: &str) -> Result<u64, TxBuilderError> {
        let amount_f64: f64 = amount_str.parse()
            .map_err(|_| TxBuilderError::InvalidAmount)?;
        
        let zatoshis = (amount_f64 * 100_000_000.0).round() as u64;
        Ok(zatoshis)
    }

    fn estimate_tx_size(&self, num_inputs: usize, num_outputs: usize) -> usize {
        10 + (num_inputs * 180) + (num_outputs * 34)
    }

    fn address_to_script_pubkey(&self, address: &str) -> Result<Script, TxBuilderError> {
        let decoded = bs58::decode(address).into_vec()
            .map_err(|_| TxBuilderError::InvalidAddress)?;

        if decoded.len() < 26 {
            return Err(TxBuilderError::InvalidAddress);
        }

        let hash160 = decoded[2..22].to_vec();
        let prefix = &decoded[0..2];
        let expected_p2pkh = self.network.p2pkh_prefix();
        let expected_p2sh = self.network.p2sh_prefix();

        if prefix == expected_p2pkh {
            Ok(bitcoin::blockdata::script::Builder::new()
                .push_opcode(bitcoin::blockdata::opcodes::all::OP_DUP)
                .push_opcode(bitcoin::blockdata::opcodes::all::OP_HASH160)
                .push_slice(&hash160[..])
                .push_opcode(bitcoin::blockdata::opcodes::all::OP_EQUALVERIFY)
                .push_opcode(bitcoin::blockdata::opcodes::all::OP_CHECKSIG)
                .into_script())
        } else if prefix == expected_p2sh {
            Ok(bitcoin::blockdata::script::Builder::new()
                .push_opcode(bitcoin::blockdata::opcodes::all::OP_HASH160)
                .push_slice(&hash160[..])
                .push_opcode(bitcoin::blockdata::opcodes::all::OP_EQUAL)
                .into_script())
        } else {
            Err(TxBuilderError::UnsupportedAddressType)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TxBuilderError {
    #[error("Invalid amount format")]
    InvalidAmount,
    #[error("Amount too small (below dust threshold)")]
    AmountTooSmall,
    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: u64, available: u64 },
    #[error("Invalid TXID format")]
    InvalidTxid,
    #[error("Invalid address format")]
    InvalidAddress,
    #[error("Unsupported address type")]
    UnsupportedAddressType,
    #[error("Invalid timelock value")]
    InvalidTimelock,
    #[error("Invalid hex encoding")]
    InvalidHex,
    #[error("Script error: {0}")]
    ScriptError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}