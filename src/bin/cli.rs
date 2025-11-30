use std::{env, sync::Arc};
use tracing::{info, Level};
use zcash_htlc_builder::{database::Database, HTLCParams, ZcashConfig, ZcashHTLCClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "create" => create_htlc(&args).await?,
        "redeem" => redeem_htlc(&args).await?,
        "refund" => refund_htlc(&args).await?,
        // "balance" => check_balance(&args).await?,
        // "utxos" => list_utxos(&args).await?,
        "keygen" => generate_keys(&args)?,
        "hashlock" => generate_hashlock(&args)?,
        "broadcast" => broadcast_tx(&args).await?,
        _ => {
            println!("❌ Unknown command: {}", command);
            print_usage();
        }
    }

    Ok(())
}

fn build_client(config_path: Option<&str>) -> Result<ZcashHTLCClient, Box<dyn std::error::Error>> {
    let config = if let Some(path) = config_path {
        info!("📄 Loading config from: {}", path);
        if path.ends_with(".json") {
            ZcashConfig::from_json_file(path)?
        } else {
            ZcashConfig::from_toml_file(path)?
        }
    } else if let Ok(env_path) = env::var("ZCASH_CONFIG") {
        info!("📄 Loading config from env: {}", env_path);
        if env_path.ends_with(".json") {
            ZcashConfig::from_json_file(&env_path)?
        } else {
            ZcashConfig::from_toml_file(&env_path)?
        }
    } else {
        info!("📄 Loading config from default locations");
        ZcashConfig::from_default_locations()?
    };

    let database = Arc::new(Database::new(
        &config.database_url,
        config.database_max_connections,
    )?);

    Ok(ZcashHTLCClient::new(config, database))
}

async fn create_htlc(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = args.get(2).map(|s| s.as_str());
    let client = build_client(config_path)?;

    info!("🔨 Creating HTLC...");

    let recipient_privkey = client.generate_privkey();
    let recipient_pubkey = client.derive_pubkey(&recipient_privkey)?;

    let refund_privkey = client.generate_privkey();
    let refund_pubkey = client.derive_pubkey(&refund_privkey)?;

    let secret = hex::encode(rand::random::<[u8; 32]>());
    let hash_lock = client.generate_hash_lock(&secret);

    info!("🔑 Recipient pubkey: {}", recipient_pubkey);
    info!("🔑 Refund pubkey: {}", refund_pubkey);
    info!("🔒 Hash lock: {}", hash_lock);
    info!("🗝️  Secret: {}", secret);

    let _params = HTLCParams {
        recipient_pubkey,
        refund_pubkey,
        hash_lock,
        timelock: 100000,
        amount: "0.01".to_string(),
    };

    info!("📝 HTLC Parameters generated successfully");
    Ok(())
}

async fn redeem_htlc(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 6 {
        println!(
            "Usage: zcash-htlc-cli redeem <htlc_id> <secret> <address> <privkey> [config_file]"
        );
        return Ok(());
    }

    let htlc_id = &args[2];
    let secret = &args[3];
    let address = &args[4];
    let privkey = &args[5];
    let config_path = args.get(6).map(|s| s.as_str());

    let client = build_client(config_path)?;

    info!("🔓 Redeeming HTLC: {}", htlc_id);
    let txid = client
        .redeem_htlc(htlc_id, secret, address, privkey)
        .await?;

    info!("✅ Redeemed! TXID: {}", txid);
    Ok(())
}

async fn refund_htlc(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 5 {
        println!("Usage: zcash-htlc-cli refund <htlc_id> <address> <privkey> [config_file]");
        return Ok(());
    }

    let htlc_id = &args[2];
    let address = &args[3];
    let privkey = &args[4];
    let config_path = args.get(5).map(|s| s.as_str());

    let client = build_client(config_path)?;

    info!("♻️ Refunding HTLC: {}", htlc_id);
    let txid = client.refund_htlc(htlc_id, address, privkey).await?;

    info!("✅ Refunded! TXID: {}", txid);
    Ok(())
}

async fn broadcast_tx(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 {
        println!("Usage: zcash-htlc-cli broadcast <hex_tx> [config_file]");
        return Ok(());
    }

    let tx_hex = &args[2];
    let config_path = args.get(3).map(|s| s.as_str());

    let client = build_client(config_path)?;
    let txid = client.broadcast_raw_tx(tx_hex).await?;

    println!("✅ Transaction broadcast!");
    println!("📋 TXID: {}", txid);

    Ok(())
}

// async fn check_balance(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
//     if args.len() < 3 {
//         println!("Usage: zcash-htlc-cli balance <address> [config_file]");
//         return Ok(());
//     }

//     let address = &args[2];
//     let config_path = args.get(3).map(|s| s.as_str());

//     let client = build_client(config_path)?;
//     let balance = client.get_balance(address).await?;

//     println!("💰 Balance: {} ZEC", balance);
//     Ok(())
// }

// async fn list_utxos(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
//     if args.len() < 3 {
//         println!("Usage: zcash-htlc-cli utxos <address> [config_file]");
//         return Ok(());
//     }

//     let address = &args[2];
//     let config_path = args.get(3).map(|s| s.as_str());

//     let client = build_client(config_path)?;
//     let utxos = client.get_utxos(address).await?;

//     println!("📦 UTXOs for {}:", address);
//     for utxo in utxos {
//         println!(
//             "  • TXID: {}, VOUT: {}, Amount: {} ZEC, Confirmations: {}",
//             utxo.txid, utxo.vout, utxo.amount, utxo.confirmations
//         );
//     }

//     Ok(())
// }

fn generate_keys(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = args.get(2).map(|s| s.as_str());
    let client = build_client(config_path)?;

    let privkey = client.generate_privkey();
    let pubkey = client.derive_pubkey(&privkey)?;

    println!("🔑 Generated Keys:");
    println!("  Private Key: {}", privkey);
    println!("  Public Key:  {}", pubkey);

    Ok(())
}

fn generate_hashlock(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 {
        println!("Usage: zcash-htlc-cli hashlock <secret> [config_file]");
        return Ok(());
    }

    let secret = &args[2];
    let config_path = args.get(3).map(|s| s.as_str());

    let client = build_client(config_path)?;
    let hash_lock = client.generate_hash_lock(secret);

    println!("🔒 Hash Lock:");
    println!("  Secret:    {}", secret);
    println!("  Hash Lock: {}", hash_lock);

    Ok(())
}

fn print_usage() {
    println!("Zcash HTLC Builder CLI");
    println!();
    println!("Usage: zcash-htlc-cli <command> [args...] [config_file]");
    println!();
    println!("Commands:");
    println!("  create [config_file]                           - Create a new HTLC");
    println!("  redeem <htlc_id> <secret> <addr> <key> [cfg]  - Redeem an HTLC");
    println!("  refund <htlc_id> <addr> <key> [cfg]           - Refund an HTLC");
    println!("  balance <address> [config_file]                - Check balance");
    println!("  utxos <address> [config_file]                  - List UTXOs");
    println!("  keygen [config_file]                           - Generate keypair");
    println!("  hashlock <secret> [config_file]                - Generate hash lock");
    println!();
    println!("Config file:");
    println!("  Use zcash-config.toml or zcash-config.json by default");
    println!("  Or specify path: zcash-htlc-cli balance <addr> ./my-config.toml");
    println!("  Or set ZCASH_CONFIG env var");
}
