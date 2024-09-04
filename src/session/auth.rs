use crate::config::Config;
use crate::transport::http_client::IGHttpClient;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, instrument};

#[derive(Debug, Serialize)]
struct AuthRequest {
    identifier: String,
    password: String,
    #[serde(rename = "encryptedPassword")]
    encrypted_password: bool,
}

// #[derive(Debug, Deserialize)]
// pub struct AuthResponse {
//     #[serde(rename = "accountId")]
//     pub account_id: String,
//     #[serde(rename = "clientId")]
//     pub client_id: String,
//     #[serde(rename = "lightstreamerEndpoint")]
//     pub lightstreamer_endpoint: String,
//     #[serde(rename = "oauthToken")]
//     pub oauth_token: Option<OAuthToken>,
//     #[serde(rename = "timezoneOffset")]
//     pub timezone_offset: f32,
// }

/*

 */
#[derive(Debug,Serialize, Deserialize)]
struct Accounts {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "accountName")]
    pub account_name: String,
    pub preferred: bool,
    #[serde(rename = "accountType")]
    pub account_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccountInfo {
    pub balance: f64,
    pub deposit: f64,
    #[serde(rename = "profitLoss")]
    pub profit_loss: f64,
    pub available: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    #[serde(rename = "accountType")]
    pub account_type: String,
    #[serde(rename = "accountInfo")]
    pub account_info: AccountInfo,
    #[serde(rename = "currencyIsoCode")]
    pub currency_iso_code: String,
    #[serde(rename = "currencySymbol")]
    pub currency_symbol: String,
    #[serde(rename = "currentAccountId")]
    pub current_account_id: String,
    #[serde(rename = "lightstreamerEndpoint")]
    pub lightstreamer_endpoint: String,
    pub accounts: Vec<Accounts>,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "timezoneOffset")]
    pub timezone_offset: i64,
    #[serde(rename = "hasActiveDemoAccounts")]
    pub has_active_demo_accounts: bool,
    #[serde(rename = "hasActiveLiveAccounts")]
    pub has_active_live_accounts: bool,
    #[serde(rename = "trailingStopsEnabled")]
    pub trailing_stops_enabled: bool,
    #[serde(rename = "reroutingEnvironment")]
    pub rerouting_environment: Option<String>,
    #[serde(rename = "dealingEnabled")]
    pub dealing_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub scope: String,
    pub token_type: String,
    pub expires_in: String,
}

#[derive(Debug, Serialize)]
struct AccountSwitchRequest {
    #[serde(rename = "accountId")]
    account_id: String,
    #[serde(rename = "defaultAccount")]
    default_account: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AccountSwitchResponse {
    #[serde(rename = "dealingEnabled")]
    dealing_enabled: bool,
    #[serde(rename = "hasActiveDemoAccounts")]
    has_active_demo_accounts: bool,
    #[serde(rename = "dealinhasActiveLiveAccountsgEnabled")]
    has_active_live_accounts: bool,
    #[serde(rename = "trailingStopsEnabled")]
    trailing_stops_enabled: bool,
}



#[derive(Debug, Deserialize)]
pub struct SessionResponse {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    pub  currency: String,
    #[serde(rename = "lightstreamerEndpoint")]
    pub lightstreamer_endpoint: String,
    pub locale: String,
    #[serde(rename = "timezoneOffset")]
    pub timezone_offset: f32,
}

#[derive(Debug)]
pub struct Session {
    client: IGHttpClient,
    config: Config,
    auth_info: Option<AuthInfo>,
}

#[derive(Debug)]
struct AuthInfo {
    auth_response: AuthResponse,
    expires_at: Instant,
    cst: String,
    x_security_token: String,
}

impl Session {
    pub fn new(config: Config) -> Result<Self> {
        let client = IGHttpClient::new(&config.rest_api.base_url, &config.credentials.api_key)?;
        Ok(Self {
            client,
            config,
            auth_info: None,
        })
    }

    #[instrument(skip(self))]
    pub async fn authenticate(&mut self, version: u8) -> Result<()> {
        if version != 2 {
            return Err(anyhow::anyhow!(
                "Unsupported authentication version: {}",
                version
            ));
        }

        debug!("Authenticating user: {}", self.config.credentials.username);

        let auth_request = AuthRequest {
            identifier: self.config.credentials.username.clone(),
            password: self.config.credentials.password.clone(),
            encrypted_password: false,
        };

        let version_header = ("version".to_string(), version.to_string());
        let token_header = (
            "x-ig-api-key".to_string(),
            self.config.credentials.api_key.clone(),
        );
        let headers = vec![version_header, token_header];

        debug!("Headers: {:?}", headers);
        let (response, cst, x_security_token) = self
            .client
            .post_with_headers::<AuthResponse, AuthRequest>("/session", &auth_request, &headers)
            .await
            .context("Failed to authenticate")?;
        debug!("Authentication response: {:?}", response);

        let auth_response = response;

        // let expires_in = if let  oauth_token = x_security_token {
        //     oauth_token.expires_in.parse::<u64>().unwrap_or(60)
        // } else {
        //     self.config.rest_api.timeout
        // };

        // self.auth_info = Some(AuthInfo {
        //     auth_response,
        //     expires_at: Instant::now() + Duration::from_secs(expires_in),
        //     cst,
        //     x_security_token,
        // });

        debug!("Authentication successful");
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn ensure_auth(&mut self) -> Result<()> {
        if let Some(auth_info) = &self.auth_info {
            if auth_info.expires_at > Instant::now() {
                return Ok(());
            }
        }

        // If we reach here, we need to reauthenticate
        self.authenticate(3).await // Default to v3 authentication
    }

    pub fn get_auth_headers(&self) -> Option<(String, String, String)> {
        // self.auth_info.as_ref().map(|info| {
        //     if let Some(ref oauth_token) = info.auth_response.oauth_token {
        //         (
        //             format!("Bearer {}", oauth_token.access_token),
        //             info.auth_response.account_id.clone(),
        //             String::new(), // No X-SECURITY-TOKEN for OAuth
        //         )
        //     } else {
        //         (
        //             info.cst.clone(),
        //             info.auth_response.account_id.clone(),
        //             info.x_security_token.clone(),
        //         )
        //     }
        // })
        None
    }

    pub async fn refresh_token(&mut self) -> Result<()> {
        // if let Some(auth_info) = &self.auth_info {
        //     if auth_info.auth_response.oauth_token.is_some() {
        //         debug!("OAuth token has expired or is about to expire. Re-authenticating...");
        //
        //         self.authenticate(3)
        //             .await
        //             .context("Failed to re-authenticate")?;
        //         return Ok(());
        //     }
        // }
        //
        // Err(anyhow::anyhow!(
        //     "No OAuth token available or session has expired"
        // ))
        Ok(())
    }

    pub async fn logout(&mut self) -> Result<()> {
        self.client
            .delete::<()>("/session")
            .await
            .context("Failed to logout")?;
        self.auth_info = None;
        Ok(())
    }

    pub async fn switch_account(
        &mut self,
        account_id: &str,
        set_default: Option<bool>,
    ) -> Result<AccountSwitchResponse> {
        let request = AccountSwitchRequest {
            account_id: account_id.to_string(),
            default_account: set_default,
        };

        let response: AccountSwitchResponse = self
            .client
            .put("/session", &request)
            .await
            .context("Failed to switch account")?;

        if let Some(auth_info) = &mut self.auth_info {
            auth_info.auth_response.current_account_id= account_id.to_string();
        }

        Ok(response)
    }

    pub async fn get_session_details(&self, fetch_session_tokens: bool) -> Result<SessionResponse> {
        let endpoint = if fetch_session_tokens {
            "/session&fetchSessionTokens=true"
        } else {
            "/session"
        };

        let response: SessionResponse = self
            .client
            .get(endpoint)
            .await
            .context("Failed to get session details")?;

        Ok(response)
    }


}

#[cfg(test)]
mod tests_session {
    use super::*;
    use mockito::Server;

    use std::time::Duration;

    fn create_test_config(server_url: &str) -> Config {
        let mut config = Config::new();
        config.rest_api.base_url = server_url.to_string();
        config.credentials.username = "test_user".to_string();
        config.credentials.password = "test_password".to_string();
        config.credentials.api_key = "test_api_key".to_string();
        config.rest_api.timeout = 3600; // 1 hora
        config
    }

    #[tokio::test]
    async fn test_authenticate_success() {
        let mut server = Server::new_async().await;
        let json_data = r#"
        {
            "clientId": "1223423",
            "accountId": "AAAAAA",
            "timezoneOffset": 1,
            "lightstreamerEndpoint": "https://demo-apd.marketdatasystems.com",
            "oauthToken": {
                "access_token": "111111",
                "refresh_token": "222222",
                "scope": "profile",
                "token_type": "Bearer",
                "expires_in": "60"
            }
        }
        "#;

        let mock = server
            .mock("POST", "/session")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json_data)
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let mut session = Session::new(config).unwrap();
        let result = session.authenticate(3).await;
        assert!(result.is_ok());
        assert!(session.auth_info.is_some());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_authenticate_failure() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/session")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let mut session = Session::new(config).unwrap();

        let result = session.authenticate(3).await;

        assert!(result.is_err());
        assert!(session.auth_info.is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_ensure_auth_when_not_authenticated() {
        let mut server = Server::new_async().await;
        let json_data = r#"
        {
            "clientId": "1223423",
            "accountId": "AAAAAA",
            "timezoneOffset": 1,
            "lightstreamerEndpoint": "https://demo-apd.marketdatasystems.com",
            "oauthToken": {
                "access_token": "111111",
                "refresh_token": "222222",
                "scope": "profile",
                "token_type": "Bearer",
                "expires_in": "60"
            }
        }
        "#;
        let mock = server
            .mock("POST", "/session")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json_data)
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let mut session = Session::new(config).unwrap();

        let result = session.ensure_auth().await;

        assert!(result.is_ok());
        assert!(session.auth_info.is_some());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_ensure_auth_when_already_authenticated() {
        let mut server = Server::new_async().await;
        let json_data = r#"
        {
            "clientId": "1223423",
            "accountId": "AAAAAA",
            "timezoneOffset": 1,
            "lightstreamerEndpoint": "https://demo-apd.marketdatasystems.com",
            "oauthToken": {
                "access_token": "111111",
                "refresh_token": "222222",
                "scope": "profile",
                "token_type": "Bearer",
                "expires_in": "60"
            }
        }
        "#;
        let mock = server
            .mock("POST", "/session")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json_data)
            .expect(1)
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let mut session = Session::new(config).unwrap();

        session.authenticate(3).await.unwrap();
        let result = session.ensure_auth().await;
        assert!(result.is_ok());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_ensure_auth_when_token_expired() {
        let mut server = Server::new_async().await;
        let json_data = r#"
        {
            "clientId": "1223423",
            "accountId": "AAAAAA",
            "timezoneOffset": 1,
            "lightstreamerEndpoint": "https://demo-apd.marketdatasystems.com",
            "oauthToken": {
                "access_token": "111111",
                "refresh_token": "222222",
                "scope": "profile",
                "token_type": "Bearer",
                "expires_in": "1"
            }
        }
        "#;
        let mock = server
            .mock("POST", "/session")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json_data)
            .expect(2)
            .create_async()
            .await;

        let mut config = create_test_config(&server.url());
        config.rest_api.timeout = 1; // Set timeout to 1 second for testing
        let mut session = Session::new(config).unwrap();

        session.authenticate(3).await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
        let result = session.ensure_auth().await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }
}

#[cfg(test)]
mod tests_auth_request_serialization {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::{json, Value}; // Mejores comparaciones en los tests

    #[test]
    fn test_auth_request_serialization() {
        let auth_request = AuthRequest {
            identifier: "testuser".to_string(),
            password: "testpassword".to_string(),
            encrypted_password: true,
        };

        let serialized = serde_json::to_string(&auth_request).unwrap();

        let serialized_value: Value = serde_json::from_str(&serialized).unwrap();

        let expected = json!({
            "identifier": "testuser",
            "password": "testpassword",
            "encryptedPassword": true
        });

        assert_eq!(serialized_value, expected);
    }
}

#[cfg(test)]
mod tests_auth_response_deserialization {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_auth_response_deserialization_with_oauth() {
        // JSON que simula la respuesta de la API con el campo `oauth_token`
        let json_data = r#"
        {
            "clientId": "1223423",
            "accountId": "AAAAAA",
            "timezoneOffset": 1,
            "lightstreamerEndpoint": "https://demo-apd.marketdatasystems.com",
            "oauthToken": {
                "access_token": "111111",
                "refresh_token": "222222",
                "scope": "profile",
                "token_type": "Bearer",
                "expires_in": "60"
            }
        }
        "#;

        let auth_response: AuthResponse = serde_json::from_str(json_data).unwrap();

        assert_eq!(auth_response.client_id, "1223423");
        assert_eq!(auth_response.account_id, "AAAAAA");
        assert_eq!(auth_response.timezone_offset, 1.0);
        assert_eq!(
            auth_response.lightstreamer_endpoint,
            "https://demo-apd.marketdatasystems.com"
        );

        let oauth_token = auth_response.oauth_token.unwrap();
        assert_eq!(oauth_token.access_token, "111111");
        assert_eq!(oauth_token.refresh_token, "222222");
        assert_eq!(oauth_token.scope, "profile");
        assert_eq!(oauth_token.token_type, "Bearer");
        assert_eq!(oauth_token.expires_in, "60");
    }

    #[test]
    fn test_auth_response_deserialization_without_oauth() {
        let json_data = r#"
        {
            "clientId": "1223423",
            "accountId": "AAAAAA",
            "timezoneOffset": 1,
            "lightstreamerEndpoint": "https://demo-apd.marketdatasystems.com"
        }
        "#;

        let auth_response: AuthResponse = serde_json::from_str(json_data).unwrap();

        assert_eq!(auth_response.client_id, "1223423");
        assert_eq!(auth_response.account_id, "AAAAAA");
        assert_eq!(auth_response.timezone_offset, 1.0);
        assert_eq!(
            auth_response.lightstreamer_endpoint,
            "https://demo-apd.marketdatasystems.com"
        );

        assert!(auth_response.oauth_token.is_none());
    }

    #[test]
    fn test_oauth_token_deserialization() {
        let json_data = r#"
        {
            "access_token": "111111",
            "refresh_token": "222222",
            "scope": "profile",
            "token_type": "Bearer",
            "expires_in": "60"
        }
        "#;

        let oauth_token: OAuthToken = serde_json::from_str(json_data).unwrap();

        assert_eq!(oauth_token.access_token, "111111");
        assert_eq!(oauth_token.refresh_token, "222222");
        assert_eq!(oauth_token.scope, "profile");
        assert_eq!(oauth_token.token_type, "Bearer");
        assert_eq!(oauth_token.expires_in, "60");
    }
}
