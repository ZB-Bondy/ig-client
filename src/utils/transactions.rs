// src/utils/transactions.rs
//
// Transaction utilities for the IG client

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use tracing::{debug, info};

use crate::{
    application::services::ig_tx_client::{IgTxClient, IgTxFetcher},
    config::Config,
    error::AppError,
    session::auth::IgAuth,
    session::interface::IgAuthenticator,
    storage::utils::store_transactions,
};

const DAYS_TO_BACK_LOOK: i64 = 10;

/// Fetch transactions from IG API and store them in the database
///
/// This function handles the entire process of:
/// 1. Authenticating with IG
/// 2. Creating a transaction client
/// 3. Fetching transactions for a date range
/// 4. Storing them in the database
///
/// # Arguments
///
/// * `cfg` - The configuration object
/// * `pool` - PostgreSQL connection pool
/// * `from_days_ago` - Optional number of days to look back (defaults to 10 days)
///
/// # Returns
///
/// * `Result<usize, AppError>` - Number of transactions inserted, or an error
///
/// # Example
///
/// ```
/// use ig_client::utils::transactions::fetch_and_store_transactions;
/// use ig_client::config::Config;
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let cfg = Config::new();
///     let pool = cfg.pg_pool().await?;
///     
///     // Fetch transactions from the last 30 days
///     let inserted = fetch_and_store_transactions(&cfg, &pool, Some(30)).await?;
///     println!("Inserted {} transactions", inserted);
///     
///     Ok(())
/// }
/// ```
pub async fn fetch_and_store_transactions(
    cfg: &Config,
    pool: &PgPool,
    from_days_ago: Option<i64>,
) -> Result<usize, AppError> {
    // Authenticate with IG
    let auth = IgAuth::new(cfg);
    let sess = auth.login().await?;
    info!("Successfully authenticated with IG");

    // Create the transaction client
    let tx_client = IgTxClient::new(cfg);
    
    // Calculate date range
    let to = Utc::now();
    let from = if let Some(days) = from_days_ago {
        to - Duration::days(days)
    } else {
        to - Duration::days(DAYS_TO_BACK_LOOK)
    };

    debug!("Fetching transactions from {} to {}", from, to);
    let txs = tx_client.fetch_range(&sess, from, to).await?;
    info!("Fetched {} transactions", txs.len());

    // Store the transactions
    let inserted = store_transactions(pool, &txs).await?;
    info!("Inserted {} rows", inserted);

    Ok(inserted)
}

/// Fetch transactions for a specific date range
///
/// This is a simpler version that only fetches transactions without storing them
///
/// # Arguments
///
/// * `cfg` - The configuration object
/// * `from` - Start date
/// * `to` - End date
///
/// # Returns
///
/// * `Result<Vec<Transaction>, AppError>` - List of transactions, or an error
pub async fn fetch_transactions(
    cfg: &Config,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<crate::application::models::transaction::Transaction>, AppError> {
    // Authenticate with IG
    let auth = IgAuth::new(cfg);
    let sess = auth.login().await?;
    debug!("Successfully authenticated with IG");

    // Create the transaction client
    let tx_client = IgTxClient::new(cfg);
    
    // Fetch transactions
    debug!("Fetching transactions from {} to {}", from, to);
    let txs = tx_client.fetch_range(&sess, from, to).await?;
    debug!("Fetched {} transactions", txs.len());

    Ok(txs)
}
