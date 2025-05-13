use ig_client::config::Config;
use ig_client::utils::logger::setup_logger;
use ig_client::utils::transactions::fetch_and_store_transactions;
use std::time::Duration as StdDuration;
use tokio::signal;
use tokio::time;
use tracing::{debug, error, info, warn};

// Maximum number of consecutive errors before forcing a cooldown
const MAX_CONSECUTIVE_ERRORS: u32 = 3;
// Cooldown time in seconds when hitting max errors
const ERROR_COOLDOWN_SECONDS: u64 = 300; // 5 minutes

const SLEEP_TIME: u64 = 24; // Sleep time in hours

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logger();
    let cfg = Config::new();
    debug!("Loaded config: database={}", cfg.database);

    // Build the Postgres pool once at startup
    let pool = cfg.pg_pool().await?;
    info!("Postgres pool established");

    // Initialize error counter
    let mut consecutive_errors = 0;

    // Set up signal handlers for graceful shutdown
    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    let hour_interval = time::interval(StdDuration::from_secs(SLEEP_TIME * 3600));
    tokio::pin!(hour_interval);

    info!("Service started, will fetch transactions hourly");

    // Immediately run once, then continue with the hourly interval
    loop {
        tokio::select! {
            _ = &mut ctrl_c => {
                info!("Received shutdown signal, terminating gracefully");
                break;
            }
            _ = hour_interval.tick() => {
                // If this is the first run, the interval will tick immediately
                info!("Starting scheduled transaction fetch");

                match fetch_and_store_transactions(&cfg, &pool, None).await {
                    Ok(inserted) => {
                        info!("Successfully processed {} transactions", inserted);
                        consecutive_errors = 0; // Reset error counter on success
                    }
                    Err(e) => {
                        error!("Error processing transactions: {}", e);
                        consecutive_errors += 1;

                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            warn!("Hit maximum consecutive errors ({}). Entering cooldown period of {} seconds",
                                  MAX_CONSECUTIVE_ERRORS, ERROR_COOLDOWN_SECONDS);

                            // Pause for cooldown period
                            time::sleep(StdDuration::from_secs(ERROR_COOLDOWN_SECONDS)).await;
                            consecutive_errors = 0; // Reset after cooldown
                        }
                    }
                }
            }
        }
    }

    info!("Service shutting down");
    Ok(())
}
