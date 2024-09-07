use serde::Deserialize;
use std::env;
use std::fmt;
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

impl fmt::Display for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"username\":\"{}\",\"password\":\"[REDACTED]\",\"account_id\":\"[REDACTED]\",\"api_key\":\"[REDACTED]\",\"client_token\":{},\"account_token\":{}}}",
               self.username,
               self.client_token.as_ref().map_or("null".to_string(), |_| "\"[REDACTED]\"".to_string()),
               self.account_token.as_ref().map_or("null".to_string(), |_| "\"[REDACTED]\"".to_string()))
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"credentials\":{},\"rest_api\":{},\"websocket\":{}}}",
            self.credentials, self.rest_api, self.websocket
        )
    }
}

impl fmt::Display for RestApiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"base_url\":\"{}\",\"timeout\":{}}}",
            self.base_url, self.timeout
        )
    }
}

impl fmt::Display for WebSocketConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"url\":\"{}\",\"reconnect_interval\":{}}}",
            self.url, self.reconnect_interval
        )
    }
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

#[cfg(test)]
mod tests_display {
    use super::*;
    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    #[test]
    fn test_credentials_display() {
        let credentials = Credentials {
            username: "user123".to_string(),
            password: "pass123".to_string(),
            account_id: "acc456".to_string(),
            api_key: "key789".to_string(),
            client_token: Some("ctoken".to_string()),
            account_token: None,
        };

        let display_output = credentials.to_string();
        let expected_json = json!({
            "username": "user123",
            "password": "[REDACTED]",
            "account_id": "[REDACTED]",
            "api_key": "[REDACTED]",
            "client_token": "[REDACTED]",
            "account_token": null
        });

        assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }

    #[test]
    fn test_rest_api_config_display() {
        let rest_api_config = RestApiConfig {
            base_url: "https://api.example.com".to_string(),
            timeout: 30,
        };

        let display_output = rest_api_config.to_string();
        let expected_json = json!({
            "base_url": "https://api.example.com",
            "timeout": 30
        });

        assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }

    #[test]
    fn test_websocket_config_display() {
        let websocket_config = WebSocketConfig {
            url: "wss://ws.example.com".to_string(),
            reconnect_interval: 5,
        };

        let display_output = websocket_config.to_string();
        let expected_json = json!({
            "url": "wss://ws.example.com",
            "reconnect_interval": 5
        });

        assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }

    #[test]
    fn test_config_display() {
        let config = Config {
            credentials: Credentials {
                username: "user123".to_string(),
                password: "pass123".to_string(),
                account_id: "acc456".to_string(),
                api_key: "key789".to_string(),
                client_token: Some("ctoken".to_string()),
                account_token: None,
            },
            rest_api: RestApiConfig {
                base_url: "https://api.example.com".to_string(),
                timeout: 30,
            },
            websocket: WebSocketConfig {
                url: "wss://ws.example.com".to_string(),
                reconnect_interval: 5,
            },
        };

        let display_output = config.to_string();
        let expected_json = json!({
            "credentials": {
                "username": "user123",
                "password": "[REDACTED]",
                "account_id": "[REDACTED]",
                "api_key": "[REDACTED]",
                "client_token": "[REDACTED]",
                "account_token": null
            },
            "rest_api": {
                "base_url": "https://api.example.com",
                "timeout": 30
            },
            "websocket": {
                "url": "wss://ws.example.com",
                "reconnect_interval": 5
            }
        });

        assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }
}
