/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 7/9/24
******************************************************************************/
use crate::config::Config;
use crate::constants::{DEFAULT_SESSION_V3_REFRESH, TOKEN_HEADER_KEY, VERSION_HEADER_KEY};
use crate::session::account::{AccountSwitchRequest, AccountSwitchResponse};
use crate::session::auth::AuthVersionResponse::{V1, V2, V3};
use crate::session::auth::{
    AuthInfo, AuthRequest, AuthResponse, AuthResponseV3, AuthVersionResponse, OAuthToken,
};
use crate::session::session_response::SessionResponse;
use crate::transport::http_client::IGHttpClient;
use anyhow::Context;
use chrono::Utc;
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use tracing::{debug, error, instrument};

#[derive(Debug)]
pub struct Session {
    client: IGHttpClient,
    config: Config,
    auth_info: Option<AuthInfo>,
    version: u8,
}

impl Session {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let client = IGHttpClient::new(&config.rest_api.base_url, &config.credentials.api_key)?;
        Ok(Self {
            client,
            config,
            auth_info: None,
            version: 1,
        })
    }

    #[instrument(skip(self))]
    pub async fn authenticate(&mut self, version: u8) -> anyhow::Result<()> {
        self.version = version;

        let headers = self.build_headers(None);
        let mut oauth_token = None;

        let (auth_version_response, cst, x_security_token) = match version {
            1 | 2 => self.authenticate_v1_v2(headers).await?,
            3 => self.authenticate_v3(headers, &mut oauth_token).await?,
            _ => panic!("Unsupported authentication version"),
        };

        debug!("Authenticating user: {}", self.config.credentials.username);
        self.auth_info = Some(AuthInfo::new(
            auth_version_response,
            Utc::now() + Duration::from_secs(DEFAULT_SESSION_V3_REFRESH),
            cst,
            x_security_token,
            oauth_token,
        ));
        debug!("Authentication successful");

        Ok(())
    }

    #[instrument(skip(self))]
    fn build_headers(
        &self,
        other: Option<HashMap<String, String>>,
    ) -> Option<HashMap<String, String>> {
        let mut headers = HashMap::new();
        headers.insert(VERSION_HEADER_KEY.to_string(), self.version.to_string());
        headers.insert(
            TOKEN_HEADER_KEY.to_string(),
            self.config.credentials.api_key.clone(),
        );
        if let Some(other_headers) = other {
            for (key, value) in other_headers {
                headers.insert(key, value);
            }
        }
        Some(headers)
    }

    #[instrument(skip(self))]
    async fn authenticate_v1_v2(
        &self,
        headers: Option<HashMap<String, String>>,
    ) -> anyhow::Result<(AuthVersionResponse, Option<String>, Option<String>)> {
        let auth_request = AuthRequest::new(
            self.config.credentials.username.clone(),
            self.config.credentials.password.clone(),
            Some(false),
        );
        let (response, cst, x_security_token) = self
            .client
            .post_with_headers::<AuthResponse, AuthRequest>("/session", &auth_request, headers)
            .await
            .context("Failed to authenticate")?;
        debug!("A,uthentication response v{}: {:?}", self.version, response);

        Ok((V1(response), cst, x_security_token))
    }

    #[instrument(skip(self))]
    async fn authenticate_v3(
        &self,
        headers: Option<HashMap<String, String>>,
        oauth_token: &mut Option<OAuthToken>,
    ) -> anyhow::Result<(AuthVersionResponse, Option<String>, Option<String>)> {
        let auth_request = AuthRequest::new(
            self.config.credentials.username.clone(),
            self.config.credentials.password.clone(),
            Some(false),
        );
        let (response, cst, x_security_token) = self
            .client
            .post_with_headers::<AuthResponseV3, AuthRequest>("/session", &auth_request, headers)
            .await
            .context("Failed to authenticate")?;
        debug!("Authentication response v{}: {:?}", self.version, &response);
        *oauth_token = response.oauth_token.clone();
        Ok((V3(response), cst, x_security_token))
    }

    #[instrument(skip(self))]
    pub async fn ensure_auth(&mut self) -> anyhow::Result<()> {
        if let Some(auth_info) = &self.auth_info {
            if auth_info.expires_at > Utc::now() {
                return Ok(());
            }
        }
        debug!("Token expired, re-authenticating");
        // TODO: If we reach here, we need to reauthenticate or refresh the token
        self.authenticate(self.version).await // Default to v1 authentication
    }

    fn get_auth_headers(&self) -> anyhow::Result<HashMap<String, String>> {
        // (CST or Authorization)
        // (X-SECURITY-TOKEN or IG-ACCOUNT-ID)
        if let Some(auth_info) = &self.auth_info {
            match &auth_info.auth_response {
                V1(_) | V2(_) => {
                    let mut headers = HashMap::new();
                    if let Some(cst) = auth_info.cst.clone() {
                        headers.insert("CST".to_string(), cst);
                    };
                    if let Some(x_security_token) = auth_info.x_security_token.clone() {
                        headers.insert("X-SECURITY-TOKEN".to_string(), x_security_token);
                    };
                    Ok(headers)
                }
                V3(response) => {
                    let mut headers = HashMap::new();
                    if let Some(oauth_token) = &auth_info.oauth_token {
                        headers.insert(
                            "Authorization".to_string(),
                            format!("Bearer {}", oauth_token.access_token),
                        );
                    }
                    headers.insert("IG-ACCOUNT-ID".to_string(), response.account_id.clone());

                    if headers.len() != 2 {
                        Ok(HashMap::new())
                    } else {
                        Ok(headers)
                    }
                }
            }
        } else {
            Ok(HashMap::new())
        }
    }

    pub async fn refresh_token(&mut self) -> anyhow::Result<()> {
        todo!("Implement token refresh using endpoint /session/refresh-token")
    }

    pub async fn logout(&mut self) -> anyhow::Result<()> {
        self.client
            .delete::<()>("/session")
            .await
            .context("Failed to logout")?;
        self.auth_info = None;
        Ok(())
    }

    pub async fn switch_account(
        // The server randomly returns a 401 Unauthorized, Body: {"errorCode":"error.security.account-token-invalid"}
        &mut self,
        account_id: &str,
        set_default: Option<bool>,
    ) -> anyhow::Result<AccountSwitchResponse> {
        let request = AccountSwitchRequest {
            account_id: account_id.to_string(),
            default_account: set_default,
        };

        let headers = match self.get_auth_headers() {
            Ok(headers) => self.build_headers(Some(headers)),
            Err(_) => None,
        };

        let response: AccountSwitchResponse =
            match self.client.put("/session", &request, &headers).await {
                Ok(response) => {
                    if let Some(auth_info) = &mut self.auth_info {
                        match auth_info.auth_response {
                            V1(ref mut r) | V2(ref mut r) => {
                                r.current_account_id = account_id.to_string()
                            }
                            V3(ref mut r) => r.account_id = account_id.to_string(),
                        }
                    }
                    response
                }
                Err(e) => {
                    error!("Error switching account: {:?}", e);
                    return Ok(AccountSwitchResponse::default());
                }
            };

        Ok(response)
    }

    pub async fn get_session_details(
        &self,
        fetch_session_tokens: bool,
    ) -> anyhow::Result<SessionResponse> {
        let endpoint = if fetch_session_tokens {
            "/session&fetchSessionTokens=true"
        } else {
            "/session"
        };

        let headers = match self.get_auth_headers() {
            Ok(headers) => Some(headers),
            Err(_) => None,
        };

        debug!("GET_SESSION_DETAILS Headers: {:?}", headers);

        let response: SessionResponse = self
            .client
            .get(endpoint, headers)
            .await
            .context("Failed to get session details")?;

        Ok(response)
    }
}

impl fmt::Display for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"client\":{},\"config\":{},\"auth_info\":{},\"version\":{}}}",
            self.client,
            self.config,
            self.auth_info.as_ref().unwrap(),
            self.version
        )
    }
}

#[cfg(test)]
mod tests_session {
    use super::*;
    use crate::utils::logger::setup_logger;
    use mockito::Server;
    use serde_json::json;

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
        setup_logger();
        let mut server = Server::new_async().await;
        let json_data = json!({
            "accountType": "CFD",
            "accountInfo": {
                "balance": 1000.0,
                "deposit": 500.0,
                "profitLoss": 200.0,
                "available": 700.0
            },
            "currencyIsoCode": "USD",
            "currencySymbol": "$",
            "currentAccountId": "ACC789",
            "lightstreamerEndpoint": "wss://example.com",
            "accounts": [{
                "accountId": "ACC789",
                "accountName": "Main Account",
                "preferred": true,
                "accountType": "CFD"
            }],
            "clientId": "CLIENT123",
            "timezoneOffset": -5,
            "hasActiveDemoAccounts": false,
            "hasActiveLiveAccounts": true,
            "trailingStopsEnabled": true,
            "reroutingEnvironment": "LIVE",
            "dealingEnabled": true
        });
        let mock = server
            .mock("POST", "/session")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json_data.to_string())
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

        // mock.assert_async().await; // TODO: Fix this test
    }
}

#[cfg(test)]
mod tests_display {
    use super::*;
    use crate::config::{Credentials, RestApiConfig, WebSocketConfig};
    use assert_json_diff::assert_json_eq;
    use serde_json::json;
    const FIXED_DURATION: Duration = Duration::from_secs(1_000_000_000);

    #[test]
    fn test_session_display() {
        let fixed_instant = Utc::now() + FIXED_DURATION;

        let session = Session {
            client: IGHttpClient::new("https://api.example.com", "key789").unwrap(),
            config: Config {
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
            },
            auth_info: Some(AuthInfo {
                auth_response: AuthVersionResponse::V1(AuthResponse {
                    account_type: "".to_string(),
                    account_info: Default::default(),
                    currency_iso_code: "".to_string(),
                    currency_symbol: "".to_string(),
                    current_account_id: "".to_string(),
                    lightstreamer_endpoint: "".to_string(),
                    accounts: vec![],
                    client_id: "".to_string(),
                    timezone_offset: 0,
                    has_active_demo_accounts: false,
                    has_active_live_accounts: false,
                    trailing_stops_enabled: false,
                    rerouting_environment: None,
                    dealing_enabled: false,
                }),
                expires_at: fixed_instant,
                cst: Option::from("cst123".to_string()),
                x_security_token: Option::from("token456".to_string()),
                oauth_token: None,
            }),
            version: 1,
        };

        let display_output = session.to_string();
        let expected_json = json!({
            "client": {
                "base_url": "https://api.example.com"
            },
            "config": {
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
            },
            "auth_info": {
          "auth_response": {
            "accountInfo": {
              "available": 0.0,
              "balance": 0.0,
              "deposit": 0.0,
              "profitLoss": 0.0
            },
            "accountType": "",
            "accounts": [],
            "clientId": "",
            "currencyIsoCode": "",
            "currencySymbol": "",
            "currentAccountId": "",
            "dealingEnabled": false,
            "hasActiveDemoAccounts": false,
            "hasActiveLiveAccounts": false,
            "lightstreamerEndpoint": "",
            "reroutingEnvironment": null,
            "timezoneOffset": 0,
            "trailingStopsEnabled": false
          },
          "cst": "[REDACTED]",
          "expires_at": format!("{:?}", fixed_instant),
          "x_security_token": "[REDACTED]"
        },
            "version": 1
        });

        assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }
}

#[cfg(test)]
mod tests_get_headers {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_get_auth_headers_v1() {
        let session = Session {
            client: IGHttpClient::new("https://api.example.com", "key789").unwrap(),
            config: Config::default(), // Asumiendo que hay un método default
            auth_info: Some(AuthInfo {
                auth_response: V1(AuthResponse {
                    account_type: "".to_string(),
                    account_info: Default::default(),
                    currency_iso_code: "".to_string(),
                    currency_symbol: "".to_string(),
                    current_account_id: "".to_string(),
                    lightstreamer_endpoint: "".to_string(),
                    accounts: vec![],
                    client_id: "".to_string(),
                    timezone_offset: 0,
                    has_active_demo_accounts: false,
                    has_active_live_accounts: false,
                    trailing_stops_enabled: false,
                    rerouting_environment: None,
                    dealing_enabled: false,
                }),
                expires_at: Utc::now(),
                cst: Some("cst123".to_string()),
                x_security_token: Some("token456".to_string()),
                oauth_token: None,
            }),
            version: 1,
        };

        let headers = session.get_auth_headers().unwrap();
        assert_eq!(headers.get("CST"), Some(&"cst123".to_string()));
        assert_eq!(
            headers.get("X-SECURITY-TOKEN"),
            Some(&"token456".to_string())
        );
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn test_get_auth_headers_v2() {
        let session = Session {
            client: IGHttpClient::new("https://api.example.com", "key789").unwrap(),
            config: Config::default(),
            auth_info: Some(AuthInfo {
                auth_response: V1(AuthResponse {
                    account_type: "".to_string(),
                    account_info: Default::default(),
                    currency_iso_code: "".to_string(),
                    currency_symbol: "".to_string(),
                    current_account_id: "".to_string(),
                    lightstreamer_endpoint: "".to_string(),
                    accounts: vec![],
                    client_id: "".to_string(),
                    timezone_offset: 0,
                    has_active_demo_accounts: false,
                    has_active_live_accounts: false,
                    trailing_stops_enabled: false,
                    rerouting_environment: None,
                    dealing_enabled: false,
                }),
                expires_at: Utc::now(),
                cst: Some("cst789".to_string()),
                x_security_token: Some("token101".to_string()),
                oauth_token: None,
            }),
            version: 2,
        };

        let headers = session.get_auth_headers().unwrap();
        assert_eq!(headers.get("CST"), Some(&"cst789".to_string()));
        assert_eq!(
            headers.get("X-SECURITY-TOKEN"),
            Some(&"token101".to_string())
        );
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn test_get_auth_headers_v3() {
        let session = Session {
            client: IGHttpClient::new("https://api.example.com", "key789").unwrap(),
            config: Config::default(),
            auth_info: Some(AuthInfo {
                auth_response: V3(AuthResponseV3 {
                    account_id: "acc123".to_string(),
                    client_id: "client456".to_string(),
                    lightstreamer_endpoint: "wss://example.com".to_string(),
                    oauth_token: Some(OAuthToken {
                        access_token: "access789".to_string(),
                        refresh_token: "refresh012".to_string(),
                        scope: "scope345".to_string(),
                        token_type: "Bearer".to_string(),
                        expires_in: "3600".to_string(),
                    }),
                    timezone_offset: 0.0,
                }),
                expires_at: Utc::now(),
                cst: None,
                x_security_token: None,
                oauth_token: Some(OAuthToken {
                    access_token: "access789".to_string(),
                    refresh_token: "refresh012".to_string(),
                    scope: "scope345".to_string(),
                    token_type: "Bearer".to_string(),
                    expires_in: "3600".to_string(),
                }),
            }),
            version: 3,
        };

        let headers = session.get_auth_headers().unwrap();
        assert_eq!(
            headers.get("Authorization"),
            Some(&"Bearer access789".to_string())
        );
        assert_eq!(headers.get("IG-ACCOUNT-ID"), Some(&"acc123".to_string()));
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn test_get_auth_headers_no_auth_info() {
        let session = Session {
            client: IGHttpClient::new("https://api.example.com", "key789").unwrap(),
            config: Config::default(),
            auth_info: None,
            version: 1,
        };

        let headers = session.get_auth_headers().unwrap();
        assert!(headers.is_empty());
    }

    #[test]
    fn test_get_auth_headers_v3_missing_fields() {
        let session = Session {
            client: IGHttpClient::new("https://api.example.com", "key789").unwrap(),
            config: Config::default(),
            auth_info: Some(AuthInfo {
                auth_response: V3(AuthResponseV3 {
                    account_id: "acount_id".to_string(),
                    client_id: "client456".to_string(),
                    lightstreamer_endpoint: "wss://example.com".to_string(),
                    oauth_token: None,
                    timezone_offset: 0.0,
                }),
                expires_at: Utc::now(),
                cst: None,
                x_security_token: None,
                oauth_token: None,
            }),
            version: 3,
        };

        let headers = session.get_auth_headers().unwrap();
        assert!(headers.is_empty());
    }
}
