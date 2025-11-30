use zcash_htlc_builder::{
    database::Database, HTLCParams, ZcashConfig, ZcashHTLCClient, UTXO,
};
use std::sync::Arc;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🧪 Testing HTLC Flow");

    // Load config
    let config = ZcashConfig::from_default_locations()?;
    let database = Arc::new(Database::new(
        &config.database_url,
        config.database_max_connections,
    )?);

    let client = ZcashHTLCClient::new(config, database);

    // ==================== Step 1: Generate Keys ====================
    info!("\n📝 Step 1: Generating Keys");

    let recipient_privkey = client.generate_privkey();
    let recipient_pubkey = client.derive_pubkey(&recipient_privkey)?;
    info!("  👤 Recipient Private Key: {}", recipient_privkey);
    info!("  👤 Recipient Public Key:  {}", recipient_pubkey);

    let refund_privkey = client.generate_privkey();
    let refund_pubkey = client.derive_pubkey(&refund_privkey)?;
    info!("  🏦 Relayer Private Key:  {}", refund_privkey);
    info!("  🏦 Relayer Public Key:   {}", refund_pubkey);

    // ==================== Step 2: Generate Secret & Hash Lock ====================
    info!("\n🔐 Step 2: Generating Secret and Hash Lock");

    let secret = hex::encode(rand::random::<[u8; 32]>());
    let hash_lock = client.generate_hash_lock(&secret);
    info!("  🗝️  Secret:    {}", secret);
    info!("  🔒 Hash Lock: {}", hash_lock);

    // ==================== Step 3: Prepare HTLC Parameters ====================
    info!("\n⚙️  Step 3: Preparing HTLC Parameters");

    let current_block = client.get_current_block_height().await?;
    let timelock = current_block + 100; // 100 blocks in the future

    let params = HTLCParams {
        recipient_pubkey: recipient_pubkey.clone(),
        refund_pubkey: refund_pubkey.clone(),
        hash_lock: hash_lock.clone(),
        timelock,
        amount: "0.001".to_string(), // 0.001 ZEC
    };

    info!("  💰 Amount:    {} ZEC", params.amount);
    info!("  ⏰ Timelock:  block {}", params.timelock);

    // ==================== Step 4: Get Funding UTXOs ====================
    info!("\n💳 Step 4: Preparing Funding (Manual - Replace with real UTXOs)");

    // TODO: Replace these with real UTXOs from your wallet
    let funding_utxos = vec![
        UTXO {
            txid: "your-txid-here".to_string(),
            vout: 0,
            script_pubkey: "your-script-pubkey-hex".to_string(),
            amount: "0.01".to_string(),
            confirmations: 6,
        }
    ];

    let change_address = "your-change-address";
    let funding_privkeys = vec!["your-funding-privkey"];

    info!("  ⚠️  Note: Update funding_utxos with real values before creating HTLC");

    // ==================== Step 5: Create HTLC (Commented - needs real UTXOs) ====================
    info!("\n🔨 Step 5: Creating HTLC (Skipped - needs real funding UTXOs)");
    
    // Uncomment when you have real UTXOs:
    /*
    let result = client.create_htlc(
        params,
        funding_utxos,
        change_address,
        funding_privkeys,
    ).await?;

    info!("  ✅ HTLC Created!");
    info!("  📋 HTLC ID:      {}", result.htlc_id);
    info!("  📋 TXID:         {}", result.txid);
    info!("  📍 P2SH Address: {}", result.p2sh_address);
    */

    // ==================== Step 6: Show Redeem Info ====================
    info!("\n🎯 Step 6: Info for User to Redeem");
    info!("  User needs to call the relayer API with:");
    info!("  {{");
    info!("    \"htlc_id\": \"<htlc-id-from-step-5>\",");
    info!("    \"secret\": \"{}\",", secret);
    info!("    \"recipient_address\": \"<user-zcash-address>\",");
    info!("    \"recipient_privkey\": \"{}\"", recipient_privkey);
    info!("  }}");

    // ==================== Summary ====================
    info!("\n📊 Summary:");
    info!("  ✅ Keys generated");
    info!("  ✅ Secret and hash lock generated");
    info!("  ✅ HTLC parameters prepared");
    info!("  ⚠️  To complete: Add real funding UTXOs and uncomment Step 5");

    Ok(())
}