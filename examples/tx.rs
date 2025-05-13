use chrono::{TimeZone, Utc};
use tracing::info;
use ig_client::application::services::ig_tx_client::{IgTxClient, IgTxFetcher};
use ig_client::config::Config;
use ig_client::session::auth::IgAuth;
use ig_client::session::interface::IgAuthenticator;
use ig_client::storage::utils::store_transactions;
use ig_client::utils::logger::setup_logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logger();
    let cfg  = Config::new();
    info!("Loaded config: database={}", cfg.database);

    // build the Postgres pool
    let pool = cfg.pg_pool().await?;
    info!("Postgres pool established");
    
    let auth = IgAuth::new(&cfg);
    let sess = auth.login().await?;

    let tx_client = IgTxClient::new(&cfg);
    let from = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let to   = Utc::now();

    let txs = tx_client.fetch_range(&sess, from, to).await?;
    info!("Fetched {} transactions", txs.len());
    let inserted = store_transactions(&pool, &txs).await?;
    info!("Inserted {} rows", inserted);
    Ok(())
}