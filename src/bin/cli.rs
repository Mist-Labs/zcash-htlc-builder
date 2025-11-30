use std::env;
use tracing::{info, Level};
use zcash_htlc_builder::{HTLCParams, ZcashHTLCClient};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "create" => create_htlc().await?,
        "redeem" => redeem_htlc(&args).await?,
        "refund" => refund_htlc(&args).await?,
        "balance" => check_balance(&args).await?,
        "utxos" => list_utxos(&args).await?,
        "keygen" => generate_keys()?,
        "hashlock" => generate_hashlock(&args)?,
        _ => {
            println!("❌ Unknown command: {}", command);
            print_usage();
        }
    }

    Ok(())
}

async fn create_htlc() -> Result<(), Box<dyn std::error::Error>> {
    info!("🔨 Creating HTLC...");

    let client = ZcashHTLCClient::from_env()?;

    // Example: Generate keys and hash lock
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
        timelock: 100000, // Example block height
        amount: "0.01".to_string(),
    };

    // You would need to provide actual funding UTXOs and private keys
    info!("⚠️  To complete creation, provide funding UTXOs and private keys");
    info!("📝 HTLC Parameters generated successfully");

    Ok(())
}

async fn redeem_htlc(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 5 {
        println!("Usage: zcash-htlc-cli redeem <htlc_id> <secret> <recipient_address> <privkey>");
        return Ok(());
    }

    let htlc_id = &args[2];
    let secret = &args[3];
    let recipient_address = &args[4];
    let privkey = &args[5];

    let client = ZcashHTLCClient::from_env()?;

    info!("🔓 Redeeming HTLC: {}", htlc_id);
    let txid = client.redeem_htlc(htlc_id, secret, recipient_address, privkey).await?;

    info!("✅ Redeemed! TXID: {}", txid);

    Ok(())
}

async fn refund_htlc(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 4 {
        println!("Usage: zcash-htlc-cli refund <htlc_id> <refund_address> <privkey>");
        return Ok(());
    }

    let htlc_id = &args[2];
    let refund_address = &args[3];
    let privkey = &args[4];

    let client = ZcashHTLCClient::from_env()?;

    info!("♻️ Refunding HTLC: {}", htlc_id);
    let txid = client.refund_htlc(htlc_id, refund_address, privkey).await?;

    info!("✅ Refunded! TXID: {}", txid);

    Ok(())
}

async fn check_balance(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 {
        println!("Usage: zcash-htlc-cli balance <address>");
        return Ok(());
    }

    let address = &args[2];
    let client = ZcashHTLCClient::from_env()?;

    let balance = client.get_balance(address).await?;
    println!("💰 Balance: {} ZEC", balance);

    Ok(())
}

async fn list_utxos(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 {
        println!("Usage: zcash-htlc-cli utxos <address>");
        return Ok(());
    }

    let address = &args[2];
    let client = ZcashHTLCClient::from_env()?;

    let utxos = client.get_utxos(address).await?;
    
    println!("📦 UTXOs for {}:", address);
    for utxo in utxos {
        println!("  • TXID: {}, VOUT: {}, Amount: {} ZEC, Confirmations: {}",
            utxo.txid, utxo.vout, utxo.amount, utxo.confirmations);
    }

    Ok(())
}

fn generate_keys() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZcashHTLCClient::from_env()?;

    let privkey = client.generate_privkey();
    let pubkey = client.derive_pubkey(&privkey)?;

    println!("🔑 Generated Keys:");
    println!("  Private Key: {}", privkey);
    println!("  Public Key:  {}", pubkey);

    Ok(())
}

fn generate_hashlock(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 {
        println!("Usage: zcash-htlc-cli hashlock <secret>");
        return Ok(());
    }

    let secret = &args[2];
    let client = ZcashHTLCClient::from_env()?;

    let hash_lock = client.generate_hash_lock(secret);

    println!("🔒 Hash Lock:");
    println!("  Secret:    {}", secret);
    println!("  Hash Lock: {}", hash_lock);

    Ok(())
}

fn print_usage() {
    println!("Zcash HTLC Builder CLI");
    println!();
    println!("Usage: zcash-htlc-cli <command> [args...]");
    println!();
    println!("Commands:");
    println!("  create                                     - Create a new HTLC");
    println!("  redeem <htlc_id> <secret> <address> <key> - Redeem an HTLC with secret");
    println!("  refund <htlc_id> <address> <key>          - Refund an HTLC after timelock");
    println!("  balance <address>                          - Check address balance");
    println!("  utxos <address>                            - List UTXOs for address");
    println!("  keygen                                     - Generate new keypair");
    println!("  hashlock <secret>                          - Generate hash lock from secret");
    println!();
    println!("Environment variables required:");
    println!("  ZCASH_NETWORK           - 'mainnet' or 'testnet'");
    println!("  ZCASH_RPC_URL           - Zcash node RPC URL");
    println!("  ZCASH_RPC_USER          - (Optional) RPC username");
    println!("  ZCASH_RPC_PASSWORD      - (Optional) RPC password");
    println!("  DATABASE_URL            - PostgreSQL connection string");
}