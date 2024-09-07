/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 4/9/24
******************************************************************************/

use anyhow::Result;
use ig_client::config::Config;
use ig_client::session::session::Session;
use ig_client::utils::logger::setup_logger;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger();

    let config = Config::new();

    let mut session = Session::new(config)?;

    match session.authenticate(3).await {
        Ok(()) => {
            info!("REST API authentication successful");
        }
        Err(e) => {
            error!("REST API authentication error: {:?}", e);
        }
    }

    // match session.get_session_details(false).await {
    //     Ok(ar) => {
    //         info!("Account details: {:?}", ar);
    //     }
    //     Err(e) => {
    //         error!("REST API get_session_details error: {:?}", e);
    //     }
    // }

    Ok(())
}
