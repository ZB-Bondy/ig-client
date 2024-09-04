/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 4/9/24
 ******************************************************************************/
use ig_client::config::Config;
use ig_client::session::auth::Session;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the logger
    tracing_subscriber::fmt::init();

    // Load the configuration
    let config = Config::new();

    // Create a session
    let mut session = Session::new(config)?;

    // Authenticate (using v3 by default)
    match session.authenticate(3).await {
        Ok(()) => {
            println!("REST API authentication successful");

            // Example: Make an authenticated request
            if let Some((auth_header, account_header)) = session.get_auth_headers() {
                // Use these headers in your HTTP client for subsequent requests
                println!("Auth Header: {}", auth_header);
                println!("Account Header: {}", account_header);

                // Here you would make your authenticated request
                // For example:
                // let balance = get_account_balance(&session).await?;
                // println!("Account balance: {}", balance);
            }
        },
        Err(e) => {
            eprintln!("REST API authentication error: {:?}", e);
        }
    }

    Ok(())
}