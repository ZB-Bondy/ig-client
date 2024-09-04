use serde::Deserialize;
use std::env;
use std::fmt::Debug;
use std::str::FromStr;
use tracing::error;

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub credentials: Credentials,
    pub base_url: String,
    pub timeout: u64,
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
                api_key: get_env_or_default("IG_API_KEY", String::from("default_api_key")),
            },
            base_url: get_env_or_default(
                "IG_BASE_URL",
                String::from("https://demo-api.ig.com/gateway/deal"),
            ),
            timeout: get_env_or_default("IG_TIMEOUT", 30),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_env_or_default() {
        assert_eq!(
            get_env_or_default::<String>("TEST_VAR_1", "default".to_string()),
            "default"
        );

        env::set_var("TEST_VAR_2", "env_value");
        assert_eq!(
            get_env_or_default::<String>("TEST_VAR_2", "default".to_string()),
            "env_value"
        );

        env::set_var("TEST_VAR_3", "not_a_number");
        assert_eq!(get_env_or_default::<i32>("TEST_VAR_3", 42), 42);
    }

    #[test]
    fn test_config_new() {
        env::set_var("IG_USERNAME", "test_user");
        env::set_var("IG_PASSWORD", "test_pass");
        env::set_var("IG_API_KEY", "test_api_key");
        env::set_var("IG_BASE_URL", "https://test-api.ig.com");
        env::set_var("IG_TIMEOUT", "60");

        let config = Config::new();

        assert_eq!(config.credentials.username, "test_user");
        assert_eq!(config.credentials.password, "test_pass");
        assert_eq!(config.credentials.api_key, "test_api_key");
        assert_eq!(config.base_url, "https://test-api.ig.com");
        assert_eq!(config.timeout, 60);
    }
}
