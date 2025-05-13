// examples/websocket_example.rs
//
// Example of using the WebSocket client to subscribe to market and account updates

use std::sync::Arc;
use tokio::time::Duration;
use tracing::info;

use ig_client::{
    config::Config, session::auth::IgAuth, session::interface::IgAuthenticator,
    transport::websocket_client::IgWebSocketClientImpl, utils::logger::setup_logger,
};
use ig_client::transport::ws_interface::IgWebSocketClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up logging
    setup_logger();

    // Load configuration
    let config = Arc::new(Config::new());
    info!("Configuration loaded");

    // Create authenticator and log in
    let authenticator = IgAuth::new(&config);
    info!("Authenticator created");

    info!("Logging in to IG...");
    let session = authenticator.login().await?;
    info!("Session started successfully");

    // Create WebSocket client
    let ws_client = IgWebSocketClientImpl::new(Arc::clone(&config));
    info!("WebSocket client created");

    // Connect to WebSocket server
    info!("Connecting to WebSocket server...");
    ws_client.connect(&session).await?;
    info!("Connected to WebSocket server");

    // Get receivers for updates
    let mut market_rx = ws_client.market_updates();
    let mut account_rx = ws_client.account_updates();

    // Subscribe to market updates for some popular markets
    // You can replace these with markets you're interested in
    let markets = vec![
        "CS.D.EURUSD.MINI.IP", // EUR/USD
        "IX.D.DAX.IFMM.IP",    // DAX
        "IX.D.FTSE.IFMM.IP",   // FTSE 100
    ];

    for market in &markets {
        info!("Subscribing to market: {}", market);
        let subscription_id = ws_client.subscribe_market(market).await?;
        info!("Subscribed with ID: {}", subscription_id);
    }

    // Subscribe to account updates
    info!("Subscribing to account updates");
    let account_sub_id = ws_client.subscribe_account().await?;
    info!("Subscribed to account updates with ID: {}", account_sub_id);

    // Process updates for 60 seconds
    info!("Listening for updates for 60 seconds...");

    let start_time = std::time::Instant::now();
    let duration = Duration::from_secs(60);

    while start_time.elapsed() < duration {
        tokio::select! {
            // Process market updates
            Some(update) = market_rx.recv() => {
                info!("Market update received: {:?}", update);
            }

            // Process account updates
            Some(update) = account_rx.recv() => {
                info!("Account update received: {:?}", update);
            }

            // Wait a bit to avoid busy-waiting
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
        }
    }

    // Unsubscribe and disconnect
    for market in &markets {
        info!("Unsubscribing from market: {}", market);
        // Note: In a real application, you would store the subscription IDs
        // Here we're just demonstrating the concept
    }

    info!("Unsubscribing from account updates");
    ws_client.unsubscribe(&account_sub_id).await?;

    info!("Disconnecting from WebSocket server");
    ws_client.disconnect().await?;
    info!("Disconnected");

    Ok(())
}
