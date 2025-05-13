//! examples/login_example.rs
//! cargo run --example login_example

use ig_client::{
    config::Config,
};
use tracing::{info, error};
use tracing_subscriber::FmtSubscriber;
use ig_client::session::auth::IgAuth;
use ig_client::session::interface::IgAuthenticator;

#[tokio::main]
async fn main() {
    // Simple console logger
    let sub = FmtSubscriber::builder().with_max_level(tracing::Level::INFO).finish();
    tracing::subscriber::set_global_default(sub).expect("setting default subscriber failed");

    // 1. Load config from env (see Config::new)
    let cfg = Config::new();
    info!("Loaded config → {}", cfg.rest_api.base_url);
    // 2. Instantiate authenticator
    let auth = IgAuth::new(&cfg);

    // 3. Try login
    match auth.login().await {
        Ok(sess) => {
            info!("✅ Auth ok. Account: {}", sess.account_id);
            println!("CST  = {}", sess.cst);
            println!("X-ST = {}", sess.token);
        }
        Err(e) => {
            error!("Auth failed: {e:?}");
        }
    }
}