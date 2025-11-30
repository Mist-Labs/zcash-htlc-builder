# Zcash HTLC Builder

[![Crates.io](https://img.shields.io/crates/v/zcash-htlc-builder.svg)](https://crates.io/crates/zcash-htlc-builder)
[![Documentation](https://docs.rs/zcash-htlc-builder/badge.svg)](https://docs.rs/zcash-htlc-builder)
[![License](https://img.shields.io/crates/l/zcash-htlc-builder.svg)](https://github.com/Mist-Labs/zcash-htlc-builder/blob/main/LICENSE)
[![Build Status](https://github.com/Mist-Labs/zcash-htlc-builder/workflows/CI/badge.svg)](https://github.com/Mist-Labs/zcash-htlc-builder/actions)

A production-ready Rust library for creating and managing Hash Time-Locked Contracts (HTLCs) on Zcash's transparent transaction layer. Built for atomic swaps and cross-chain bridges.

## 🌟 Features

- ✅ **ZIP-300 Compliant** - Full HTLC script implementation
- ✅ **Bitcoin 0.29 Compatible** - Works with Zcash transparent transactions
- ✅ **Database Persistence** - PostgreSQL with Diesel ORM
- ✅ **Block Explorer Integration** - Query UTXOs without running a full node
- ✅ **CLI Tool** - Command-line interface for testing and operations
- ✅ **Type-Safe** - Full Rust type safety with comprehensive error handling
- ✅ **Async/Await** - Modern async Rust with Tokio
- ✅ **Config File Support** - TOML/JSON configuration (no environment variables required)

## 📦 Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
zcash-htlc-builder = "0.1.5"
tokio = { version = "1", features = ["full"] }
```

## 🚀 Quick Start

### 1. Setup Configuration

Create `zcash-config.toml` in your project root:
```toml
network = "Testnet"  # or "Mainnet"
rpc_url = "http://localhost:18232"
rpc_user = "your-rpc-user"
rpc_password = "your-rpc-password"
database_url = "postgresql://user:password@localhost/zcash_htlc"
database_max_connections = 10
explorer_api = "https://explorer.testnet.z.cash/api"

# Optional: Relayer Configuration (for automated HTLC management)
[relayer]
hot_wallet_privkey = "your-private-key"
hot_wallet_address = "your-zcash-address"
max_tx_per_batch = 10
poll_interval_secs = 10
max_retry_attempts = 3
min_confirmations = 1
network_fee_zec = "0.0001"
```

> **⚠️ Security Warning:** Never commit `zcash-config.toml` with real credentials to version control. Add it to `.gitignore` and use `zcash-config.toml.example` as a template.

### 2. Setup Database
```bash
# Create PostgreSQL database
createdb zcash_htlc

# Migrations run automatically when you first use the library
```

### 3. Basic Usage
```rust
use zcash_htlc_builder::{
    ZcashHTLCClient, ZcashConfig, HTLCParams, UTXO,
    database::Database,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from file
    let config = ZcashConfig::from_toml_file("zcash-config.toml")?;
    
    // Initialize database
    let database = Arc::new(Database::new(
        &config.database_url,
        config.database_max_connections,
    )?);
    
    // Create client
    let client = ZcashHTLCClient::new(config, database);

    // Generate keys
    let recipient_privkey = client.generate_privkey();
    let recipient_pubkey = client.derive_pubkey(&recipient_privkey)?;
    
    let refund_privkey = client.generate_privkey();
    let refund_pubkey = client.derive_pubkey(&refund_privkey)?;

    // Generate secret and hash lock
    let secret = hex::encode(rand::random::<[u8; 32]>());
    let hash_lock = client.generate_hash_lock(&secret);

    // Create HTLC parameters
    let params = HTLCParams {
        recipient_pubkey,
        refund_pubkey,
        hash_lock: hash_lock.clone(),
        timelock: 500000, // Block height
        amount: "0.01".to_string(),
    };

    // Prepare funding (replace with your actual UTXOs)
    let funding_utxos = vec![
        UTXO {
            txid: "your-txid".to_string(),
            vout: 0,
            script_pubkey: "script-hex".to_string(),
            amount: "0.02".to_string(),
            confirmations: 6,
        }
    ];

    // Create HTLC
    let result = client.create_htlc(
        params,
        funding_utxos,
        "your-change-address",
        vec!["your-funding-privkey"],
    ).await?;

    println!("✅ HTLC Created!");
    println!("📋 HTLC ID: {}", result.htlc_id);
    println!("📋 TXID: {}", result.txid);
    println!("📍 P2SH Address: {}", result.p2sh_address);
    println!("🗝️  Secret: {}", secret);

    // Later, redeem the HTLC
    let redeem_txid = client.redeem_htlc(
        &result.htlc_id,
        &secret,
        "recipient-address",
        &recipient_privkey,
    ).await?;

    println!("✅ HTLC Redeemed: {}", redeem_txid);

    Ok(())
}
```

### Alternative: JSON Configuration

You can also use JSON format:
```json
{
  "network": "testnet",
  "rpc_url": "http://localhost:18232",
  "rpc_user": "user",
  "rpc_password": "password",
  "database_url": "postgresql://localhost/zcash_htlc",
  "database_max_connections": 10,
  "explorer_api": "https://explorer.testnet.z.cash/api"
}
```

Load with:
```rust
let config = ZcashConfig::from_json_file("zcash-config.json")?;
```

## 🛠️ CLI Tool

The library includes a command-line tool for testing and operations.

### Installation
```bash
cargo install zcash-htlc-builder
```

### Usage

All CLI commands use the config file from your project root or via custom path.

#### Generate Keys
```bash
zcash-htlc-cli keygen
# Or with custom config:
zcash-htlc-cli keygen ./my-config.toml
```

#### Generate Hash Lock
```bash
zcash-htlc-cli hashlock "my-secret-phrase"
```

**Output:**
```
🔒 Hash Lock:
  Secret:    my-secret-phrase
  Hash Lock: 6e9f78c1c24acdee688a360f1212c9b9989e7469d6a6e39e4ed7ca279f0c7846
```

#### Create HTLC
```bash
zcash-htlc-cli create
```

#### Redeem HTLC
```bash
zcash-htlc-cli redeem <htlc_id> <secret> <recipient_address> <privkey>
```

#### Refund HTLC
```bash
zcash-htlc-cli refund <htlc_id> <refund_address> <privkey>
```

#### Broadcast Raw Transaction
```bash
zcash-htlc-cli broadcast <hex-encoded-tx>
```

### Environment Variable Override

You can set `ZCASH_CONFIG` environment variable to specify config file location:
```bash
export ZCASH_CONFIG=./production-config.toml
zcash-htlc-cli keygen
```

## 📚 Examples

Check the `examples/` directory for complete working examples:
```bash
# Run the full HTLC flow example
cargo run --example test_htlc_flow
```

**Output:**
```
🧪 Testing HTLC Flow
📝 Step 1: Generating Keys
  👤 Recipient Private Key: 489175b18cd8f36e...
  👤 Recipient Public Key:  02f6b9fc88bf40a5c...
  🏦 Relayer Private Key:  c980fd0225a51ec8...
  🏦 Relayer Public Key:   03bb93b3c358ba56e...
🔐 Step 2: Generating Secret and Hash Lock
  🗝️  Secret:    8f2c59d64b11f329...
  🔒 Hash Lock: 346a7d2128ff4d1b...
...
```

## 🏗️ Architecture
```
┌─────────────────────────────────────────────┐
│         ZcashHTLCClient (Main API)          │
├─────────────────────────────────────────────┤
│  • create_htlc()                            │
│  • redeem_htlc()                            │
│  • refund_htlc()                            │
│  • generate_privkey()                       │
│  • derive_pubkey()                          │
│  • generate_hash_lock()                     │
└─────────────────────────────────────────────┘
          │           │           │
          ▼           ▼           ▼
┌──────────────┐ ┌──────────┐ ┌──────────────┐
│   Builder    │ │  Signer  │ │   Database   │
│              │ │          │ │              │
│ • HTLC TX    │ │ • Sign   │ │ • HTLCs      │
│ • Redeem TX  │ │ • Verify │ │ • Operations │
│ • Refund TX  │ │ • Keys   │ │ • UTXOs      │
└──────────────┘ └──────────┘ └──────────────┘
```

## 💾 Database Schema

| Table | Description |
|-------|-------------|
| **zcash_htlcs** | HTLC state and metadata |
| **htlc_operations** | Transaction operations (create/redeem/refund) |
| **relayer_utxos** | UTXOs managed by relayer's hot wallet |
| **indexer_checkpoints** | Blockchain sync state |

## ⚙️ Configuration Options

### Core Configuration

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `network` | string | ✅ Yes | "testnet" or "mainnet" |
| `rpc_url` | string | ✅ Yes | Zcash RPC endpoint |
| `rpc_user` | string | ❌ No | RPC username or API key |
| `rpc_password` | string | ❌ No | RPC password |
| `database_url` | string | ✅ Yes | PostgreSQL connection string |
| `database_max_connections` | number | ❌ No | Max DB connections (default: 10) |
| `explorer_api` | string | ❌ No | Block explorer API URL |

### Relayer Configuration (Optional)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `hot_wallet_privkey` | string | ⚠️ Yes* | Private key for funding |
| `hot_wallet_address` | string | ⚠️ Yes* | Address for funding |
| `max_tx_per_batch` | number | ❌ No | Max transactions per batch (default: 10) |
| `poll_interval_secs` | number | ❌ No | Polling interval in seconds (default: 10) |
| `network_fee_zec` | string | ❌ No | Network fee in ZEC (default: "0.0001") |

*Required only if running automated relayer

## 🔒 Security Considerations

### Private Key Management

- ⛔ **Never** commit `zcash-config.toml` with real keys to version control
- 🔐 Use hardware wallets for production mainnet operations
- 🗄️ Store keys securely (HSM, encrypted storage, environment secrets)
- 🔄 Rotate keys regularly

### Timelock Safety

- ⏰ Always set timelocks with sufficient buffer (consider network congestion)
- 📊 Monitor block height before attempting refunds
- ✅ Account for at least 6 confirmations

### Transaction Verification

- 🔍 Always verify transactions before signing
- 💰 Check amounts, addresses, and scripts carefully
- 🧪 Test on testnet first

### Database Security

- 🔑 Use strong PostgreSQL credentials
- 🔐 Enable SSL for database connections in production
- 💾 Regularly backup database

## 🌐 Network Configuration

### Testnet

| Property | Value |
|----------|-------|
| **RPC Port** | `18232` |
| **Faucet** | [testnet.zecfaucet.com](https://testnet.zecfaucet.com) |
| **Explorer** | [blockexplorer.one/zcash/testnet](https://blockexplorer.one/zcash/testnet) |

### Mainnet

| Property | Value |
|----------|-------|
| **RPC Port** | `8232` |
| **Explorer** | [blockexplorer.one/zcash/mainnet](https://blockexplorer.one/zcash/mainnet) |

## 🧪 Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_build_htlc_script

# With logging
RUST_LOG=debug cargo test

# Run examples
cargo run --example test_htlc_flow
```

## 📦 Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| **bitcoin** | 0.29 | Transaction building (Zcash compatible) |
| **secp256k1** | 0.24 | Cryptographic signatures |
| **diesel** | 2.1 | PostgreSQL ORM |
| **tokio** | 1.0 | Async runtime |
| **reqwest** | 0.11 | HTTP client for RPC |
| **serde** | 1.0 | Serialization/deserialization |
| **toml** | 0.8 | TOML configuration parsing |

## 🐛 Troubleshooting

### "Failed to read config file"

- ✅ Ensure `zcash-config.toml` exists in project root
- ✅ Check file permissions
- ✅ Verify TOML syntax with a [validator](https://www.toml-lint.com/)

### "Database connection failed"

- ✅ Verify PostgreSQL is running: `pg_isready`
- ✅ Check database credentials in config
- ✅ Ensure database exists: `createdb zcash_htlc`

### "RPC connection failed"

- ✅ Check RPC credentials
- ✅ Ensure correct port (18232 for testnet, 8232 for mainnet)

### "HTLC creation failed"

- ✅ Verify sufficient balance in funding UTXOs
- ✅ Check that UTXOs are confirmed (at least 1 confirmation)
- ✅ Ensure private keys match funding addresses

## 🤝 Contributing

Contributions welcome! Please:

1. 🍴 Fork the repository
2. 🌿 Create a feature branch (`git checkout -b feature/amazing-feature`)
3. ✅ Write tests for new functionality
4. 🧪 Ensure `cargo test` and `cargo clippy` pass
5. 📝 Update documentation
6. 🚀 Submit a pull request

### Development Setup
```bash
# Clone repository
git clone https://github.com/Mist-Labs/zcash-htlc-builder.git
cd zcash-htlc-builder

# Install dependencies
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## 📋 Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.

## 📄 License

This project is licensed under the Apache-2.0 License - see the [LICENSE](LICENSE) file for details.

## 📚 Resources

- 📖 [ZIP-300: Cross-chain Atomic Transactions](https://zips.z.cash/zip-0300)
- 📖 [Zcash Protocol Specification](https://zips.z.cash/protocol/protocol.pdf)
- 📖 [Bitcoin Developer Reference](https://developer.bitcoin.org/reference/)
- 📖 [Diesel ORM Guide](https://diesel.rs/guides/)

## 💬 Support

- 📝 **Issues**: [GitHub Issues](https://github.com/Mist-Labs/zcash-htlc-builder/issues)
- 📚 **Docs**: [docs.rs/zcash-htlc-builder](https://docs.rs/zcash-htlc-builder)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/Mist-Labs/zcash-htlc-builder/discussions)

## 🙏 Acknowledgments

Built with ❤️ for the Zcash ecosystem by [Mist Labs](https://github.com/Mist-Labs)

---

**⭐ If you find this library useful, please star the repository!**
