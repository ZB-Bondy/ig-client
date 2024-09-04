use serde::Deserialize;
use std::env;
use std::fmt::Debug;
use std::str::FromStr;
use tracing::error;

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub(crate) account_id: String,
    pub api_key: String,
    pub(crate) client_token: Option<String>,
    pub(crate) account_token: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub credentials: Credentials,
    pub rest_api: RestApiConfig,
    pub websocket: WebSocketConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RestApiConfig {
    pub base_url: String,
    pub timeout: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebSocketConfig {
    pub url: String,
    pub reconnect_interval: u64,
}

pub fn get_env_or_default<T: FromStr>(env_var: &str, default: T) -> T
where
    <T as FromStr>::Err: Debug,
{
    match env::var(env_var) {
        Ok(val) => val.parse::<T>().unwrap_or_else(|_| {
            error!("Failed to parse {}: {}, using default", env_var, val);
            default
        }),
        Err(_) => default,
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Self {
        Config {
            credentials: Credentials {
                username: get_env_or_default("IG_USERNAME", String::from("default_username")),
                password: get_env_or_default("IG_PASSWORD", String::from("default_password")),
                account_id: get_env_or_default("IG_ACCOUNT_ID", String::from("default_account_id")),
                api_key: get_env_or_default("IG_API_KEY", String::from("default_api_key")),
                client_token: None,
                account_token: None,
            },
            rest_api: RestApiConfig {
                base_url: get_env_or_default(
                    "IG_REST_BASE_URL",
                    String::from("https://demo-api.ig.com/gateway/deal"),
                ),
                timeout: get_env_or_default("IG_REST_TIMEOUT", 30),
            },
            websocket: WebSocketConfig {
                url: get_env_or_default(
                    "IG_WS_URL",
                    String::from("wss://demo-apd.marketdatasystems.com"),
                ),
                reconnect_interval: get_env_or_default("IG_WS_RECONNECT_INTERVAL", 5),
            },
        }
    }
}

#[cfg(test)]
mod tests_config {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static ENV_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn with_env_vars<F>(vars: Vec<(&str, &str)>, test: F)
    where
        F: FnOnce(),
    {
        let _lock = ENV_MUTEX.lock().unwrap();
        let mut old_vars = Vec::new();

        for (key, value) in vars {
            old_vars.push((key, env::var(key).ok()));
            env::set_var(key, value);
        }

        test();

        for (key, value) in old_vars {
            match value {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }
    }

    #[test]
    fn test_config_new() {
        with_env_vars(
            vec![
                ("IG_USERNAME", "test_user"),
                ("IG_PASSWORD", "test_pass"),
                ("IG_API_KEY", "test_api_key"),
                ("IG_REST_BASE_URL", "https://test-api.ig.com"),
                ("IG_REST_TIMEOUT", "60"),
                ("IG_WS_URL", "wss://test-ws.ig.com"),
                ("IG_WS_RECONNECT_INTERVAL", "10"),
            ],
            || {
                let config = Config::new();

                assert_eq!(config.credentials.username, "test_user");
                assert_eq!(config.credentials.password, "test_pass");
                assert_eq!(config.credentials.api_key, "test_api_key");
                assert_eq!(config.rest_api.base_url, "https://test-api.ig.com");
                assert_eq!(config.rest_api.timeout, 60);
                assert_eq!(config.websocket.url, "wss://test-ws.ig.com");
                assert_eq!(config.websocket.reconnect_interval, 10);
            },
        );
    }

    #[test]
    fn test_default_values() {
        with_env_vars(vec![], || {
            let config = Config::new();

            assert_eq!(config.credentials.username, "default_username");
            assert_eq!(config.credentials.password, "default_password");
            assert_eq!(config.credentials.api_key, "default_api_key");
            assert_eq!(
                config.rest_api.base_url,
                "https://demo-api.ig.com/gateway/deal"
            );
            assert_eq!(config.rest_api.timeout, 30);
            assert_eq!(config.websocket.url, "wss://demo-apd.marketdatasystems.com");
            assert_eq!(config.websocket.reconnect_interval, 5);
        });
    }
}
