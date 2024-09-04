/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 4/9/24
 ******************************************************************************/

use ig_client::config::Config;
use ig_client::session::ws_auth::WSAuthSession;
use std::sync::Arc;
use tokio;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the logger
    tracing_subscriber::fmt::init();

    // Load the configuration
    let config = Arc::new(Config::new());

    // Create a WebSocket authentication session
    let mut ws_auth_session = WSAuthSession::new(config.clone())?;

    // Attempt to authenticate
    match ws_auth_session.authenticate().await {
        Ok(session_id) => {
            println!("Authentication successful. Session ID: {}", session_id);

            // Here you could continue with other operations using the authenticated WebSocket client
            let ws_client = ws_auth_session.get_client();

            // Example: send a message after authentication
            ws_client.send("Post-authentication test message".to_string()).await?;

            // Here you could implement a loop to handle incoming messages
            // For example:
            // while let Some(message) = rx.recv().await {
            //     println!("Received message: {}", message);
            // }
        },
        Err(e) => {
            eprintln!("Authentication error: {:?}", e);
        }
    }

    Ok(())
}