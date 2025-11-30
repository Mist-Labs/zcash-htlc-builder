# Zcash HTLC Builder

A production-ready Rust library for creating and managing Hash Time-Locked Contracts (HTLCs) on Zcash's transparent transaction layer. Built for atomic swaps and cross-chain bridges.

## Features

- ✅ **ZIP-300 Compliant** - Full HTLC script implementation
- ✅ **Bitcoin 0.29 Compatible** - Works with Zcash transparent transactions
- ✅ **Database Persistence** - PostgreSQL with Diesel ORM
- ✅ **Block Explorer Integration** - Query UTXOs without running a full node
- ✅ **CLI Tool** - Command-line interface for testing and operations
- ✅ **Type-Safe** - Full Rust type safety with comprehensive error handling
- ✅ **Async/Await** - Modern async Rust with Tokio

## Installation

Add to your `Cargo.toml`:
```toml
zcash-htlc-builder = "0.1.3"
```

## Quick Start
1. Setup Configuration
Create a file named `zcash-config.toml` in your project root with your configuration.

```toml
network = "Testnet" # OR Mainnet 
rpc_url = "http://localhost:18232"
rpc_user = "user" # Can be RPC API-KEY
database_url = "postgres://localhost/zcash"
database_max_connections = 10
explorer_api = "https://explorer.testnet.z.cash/api"

# Relayer Configuration
[relayer]
hot_wallet_privkey = "privKey"
hot_wallet_address = "address" # Relayer's funding address

# Relayer settings
max_tx_per_batch = 10
poll_interval_secs = 10
max_retry_attempts = 3
min_confirmations = 1

# Fee settings
network_fee_zec = "0.0001"
```

### 2. Setup Database
```bash
# Create PostgreSQL database
createdb zcash_htlc

# Run migrations (automatic in dev mode, uses database_url from zcash-config.toml)
cargo run --bin zcash-htlc-cli
```

### 3. Basic Usage
```rust
use zcash_htlc_builder::{ZcashHTLCClient, HTLCParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client from environment
    let client = ZcashHTLCClient::from_env()?;

    // Generate keys
    let recipient_privkey = client.generate_privkey();
    let recipient_pubkey = client.derive_pubkey(&recipient_privkey)?;
    
    let refund_privkey = client.generate_privkey();
    let refund_pubkey = client.derive_pubkey(&refund_privkey)?;

    // Generate secret and hash lock
    let secret = hex::encode(rand::random::<[u8; 32]>());
    let hash_lock = client.generate_hash_lock(&secret);

    // Create HTLC
    let params = HTLCParams {
        recipient_pubkey,
        refund_pubkey,
        hash_lock,
        timelock: 500000, // Block height
        amount: "0.01".to_string(),
    };

    let result = client.create_htlc(
        params,
        funding_utxos,
        change_address,
        funding_privkeys,
    ).await?;

    println!("HTLC Created: {}", result.txid);
    println!("P2SH Address: {}", result.p2sh_address);

    Ok(())
}
```

## CLI Tool

### Generate Keys
```bash
zcash-htlc-cli keygen
```

### Create HTLC
```bash
zcash-htlc-cli create
```

### Redeem HTLC
```bash
zcash-htlc-cli redeem <htlc_id> <secret> <recipient_address> <privkey>
```

### Refund HTLC
```bash
zcash-htlc-cli refund <htlc_id> <refund_address> <privkey>
```

## Architecture
```
┌─────────────────────────────────────────────┐
│         ZcashHTLCClient (Main API)          │
├─────────────────────────────────────────────┤
│  • create_htlc()                            │
│  • redeem_htlc()                            │
│  • refund_htlc()                            │
│  • get_utxos() / get_balance()              │
└─────────────────────────────────────────────┘
          │           │           │
          ▼           ▼           ▼
┌──────────────┐ ┌──────────┐ ┌──────────────┐
│   Builder    │ │  Signer  │ │   Database   │
│              │ │          │ │              │
│ • HTLC TX    │ │ • Sign   │ │ • HTLCs      │
│ • Redeem TX  │ │ • Verify │ │ • Operations │
│ • Refund TX  │ │ • Keys   │ │ • Checkpoints│
└──────────────┘ └──────────┘ └──────────────┘
```

## Database Schema

- **zcash_htlcs** - HTLC state and metadata
- **htlc_operations** - Transaction operations (create/redeem/refund)
- **relayer_utxos** - Unspent Transaction Outputs (UTXOs) managed by the Relayer's hot wallet for funding operations.
- **indexer_checkpoints** - Blockchain sync state

## Security Considerations

⚠️ **Private Key Management**
- Never commit private keys to version control
- Use hardware wallets for production
- Store keys securely (HSM, encrypted storage)

⚠️ **Timelock Safety**
- Always set timelocks with sufficient buffer
- Monitor block height before refunds
- Account for network congestion

⚠️ **Transaction Verification**
- Always verify transactions before signing
- Check amounts, addresses, and scripts
- Use testnet for testing

## Network Configuration

### Testnet
- RPC Port: 18232
- FFaucet: https://testnet.zecfaucet.com/
- Explorer: https://blockexplorer.one/zcash/testnet

### Mainnet
- RPC Port: 8232
- Explorer: https://blockexplorer.one/zcash/mainnet

## Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_build_htlc_script

# With logging
RUST_LOG=info cargo test
```

## Dependencies

- **bitcoin 0.29** - Transaction building (Zcash compatible)
- **secp256k1 0.24** - Cryptographic signatures
- **diesel 2.1** - PostgreSQL ORM
- **tokio 1.0** - Async runtime
- **reqwest 0.11** - HTTP client for RPC

## Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Write tests for new functionality
4. Ensure `cargo test` passes
5. Submit a pull request

## License

Apache-2.0

## Resources

- [ZIP-300: Cross-chain Atomic Transactions](https://zips.z.cash/zip-0300)
- [Zcash Protocol Specification](https://zips.z.cash/protocol/protocol.pdf)
- [Bitcoin Developer Reference](https://developer.bitcoin.org/reference/)

## Support

- Issues: https://github.com/Mist-Labs/zcash-htlc-builder/issues
- Docs: https://docs.rs/zcash-htlc-builder

---

Built with ❤️ for the Zcash ecosystem
