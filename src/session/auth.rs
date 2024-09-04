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

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub client_token: String,
    pub account_token: String,
    pub lightstreamer_endpoint: String,
    pub cst: String,
    pub x_security_token: String,
    #[serde(rename = "oauthToken")]
    pub oauth_token: Option<OAuthToken>,
}

#[derive(Debug, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub scope: String,
    pub token_type: String,
    pub expires_in: String,
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
        debug!("Authenticating user: {}", self.config.credentials.username);

        let auth_request = AuthRequest {
            identifier: self.config.credentials.username.clone(),
            password: self.config.credentials.password.clone(),
            encrypted_password: false,
        };

        let response: AuthResponse = self.client
            .post(&format!("/session?version={}", version), &auth_request)
            .await
            .context("Failed to authenticate")?;

        let expires_in = if let Some(ref oauth_token) = response.oauth_token {
            oauth_token.expires_in.parse::<u64>().unwrap_or(60)
        } else {
            self.config.rest_api.timeout
        };

        self.auth_info = Some(AuthInfo {
            auth_response: response,
            expires_at: Instant::now() + Duration::from_secs(expires_in),
        });

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

    pub fn get_auth_headers(&self) -> Option<(String, String)> {
        self.auth_info.as_ref().map(|info| {
            if let Some(ref oauth_token) = info.auth_response.oauth_token {
                (
                    format!("Bearer {}", oauth_token.access_token),
                    self.config.credentials.account_id.clone(),
                )
            } else {
                (
                    info.auth_response.cst.clone(),
                    info.auth_response.x_security_token.clone(),
                )
            }
        })
    }

    pub async fn refresh_token(&mut self) -> Result<()> {
        if let Some(auth_info) = &self.auth_info {
            if let Some(ref _oauth_token) = auth_info.auth_response.oauth_token {
                // Implement token refresh logic here
                // This should make a request to the token refresh endpoint
                // and update the auth_info with the new tokens
                todo!("Implement token refresh")
            }
        }
        Err(anyhow::anyhow!("No OAuth token available for refresh"))
    }
}

// #[cfg(test)]
// mod tests_session {
//     use super::*;
//     use mockito::Server;
//     use pretty_assertions::assert_eq;
//     use std::time::Duration;
//
//     fn create_test_config(server_url: &str) -> Config {
//         let mut config = Config::new();
//         config.rest_api.base_url = server_url.to_string();
//         config.credentials.username = "test_user".to_string();
//         config.credentials.password = "test_password".to_string();
//         config.credentials.api_key = "test_api_key".to_string();
//         config.rest_api.timeout = 3600; // 1 hora
//         config
//     }
//
//     #[tokio::test]
//     async fn test_authenticate_success() {
//         let mut server = Server::new_async().await;
//         let mock = server
//             .mock("POST", "/session")
//             .with_status(200)
//             .with_header("content-type", "application/json")
//             .with_body(
//                 r#"
//                 {
//                     "client_token": "test_client_token",
//                     "account_token": "test_account_token",
//                     "lightstreamer_endpoint": "https://test.lightstreamer.com",
//                     "cst": "test_cst",
//                     "x_security_token": "test_x_security_token"
//                 }
//             "#,
//             )
//             .create_async()
//             .await;
//
//         let config = create_test_config(&server.url());
//         let mut session = Session::new(config).unwrap();
//
//         let result = session.authenticate().await;
//
//         assert!(result.is_ok());
//         assert!(session.auth_info.is_some());
//
//         if let Some(auth_info) = &session.auth_info {
//             assert_eq!(auth_info.auth_response.client_token, "test_client_token");
//             assert_eq!(auth_info.auth_response.cst, "test_cst");
//             assert_eq!(
//                 auth_info.auth_response.x_security_token,
//                 "test_x_security_token"
//             );
//         }
//
//         mock.assert_async().await;
//     }
//
//     #[tokio::test]
//     async fn test_authenticate_failure() {
//         let mut server = Server::new_async().await;
//         let mock = server
//             .mock("POST", "/session")
//             .with_status(401)
//             .with_body("Unauthorized")
//             .create_async()
//             .await;
//
//         let config = create_test_config(&server.url());
//         let mut session = Session::new(config).unwrap();
//
//         let result = session.authenticate().await;
//
//         assert!(result.is_err());
//         assert!(session.auth_info.is_none());
//
//         mock.assert_async().await;
//     }
//
//     #[tokio::test]
//     async fn test_ensure_auth_when_not_authenticated() {
//         let mut server = Server::new_async().await;
//         let mock = server
//             .mock("POST", "/session")
//             .with_status(200)
//             .with_header("content-type", "application/json")
//             .with_body(
//                 r#"
//                 {
//                     "client_token": "test_client_token",
//                     "account_token": "test_account_token",
//                     "lightstreamer_endpoint": "https://test.lightstreamer.com",
//                     "cst": "test_cst",
//                     "x_security_token": "test_x_security_token"
//                 }
//             "#,
//             )
//             .create_async()
//             .await;
//
//         let config = create_test_config(&server.url());
//         let mut session = Session::new(config).unwrap();
//
//         let result = session.ensure_auth().await;
//
//         assert!(result.is_ok());
//         assert!(session.auth_info.is_some());
//
//         mock.assert_async().await;
//     }
//
//     #[tokio::test]
//     async fn test_ensure_auth_when_already_authenticated() {
//         let mut server = Server::new_async().await;
//         let mock = server
//             .mock("POST", "/session")
//             .with_status(200)
//             .with_header("content-type", "application/json")
//             .with_body(
//                 r#"
//                 {
//                     "client_token": "test_client_token",
//                     "account_token": "test_account_token",
//                     "lightstreamer_endpoint": "https://test.lightstreamer.com",
//                     "cst": "test_cst",
//                     "x_security_token": "test_x_security_token"
//                 }
//             "#,
//             )
//             .expect(1)
//             .create_async()
//             .await;
//
//         let config = create_test_config(&server.url());
//         let mut session = Session::new(config).unwrap();
//
//         session.authenticate().await.unwrap();
//         let result = session.ensure_auth().await;
//         assert!(result.is_ok());
//
//         mock.assert_async().await;
//     }
//
//     #[tokio::test]
//     async fn test_ensure_auth_when_token_expired() {
//         let mut server = Server::new_async().await;
//         let mock = server
//             .mock("POST", "/session")
//             .with_status(200)
//             .with_header("content-type", "application/json")
//             .with_body(
//                 r#"
//                 {
//                     "client_token": "test_client_token",
//                     "account_token": "test_account_token",
//                     "lightstreamer_endpoint": "https://test.lightstreamer.com",
//                     "cst": "test_cst",
//                     "x_security_token": "test_x_security_token"
//                 }
//             "#,
//             )
//             .expect(2)
//             .create_async()
//             .await;
//
//         let mut config = create_test_config(&server.url());
//         config.rest_api.timeout = 1; // Set timeout to 1 second for testing
//         let mut session = Session::new(config).unwrap();
//
//         session.authenticate().await.unwrap();
//         tokio::time::sleep(Duration::from_secs(2)).await;
//         let result = session.ensure_auth().await;
//         assert!(result.is_ok());
//         mock.assert_async().await;
//     }
//
//     #[tokio::test]
//     async fn test_get_auth_tokens() {
//         let mut server = Server::new_async().await;
//         let mock = server
//             .mock("POST", "/session")
//             .with_status(200)
//             .with_header("content-type", "application/json")
//             .with_body(
//                 r#"
//                 {
//                     "client_token": "test_client_token",
//                     "account_token": "test_account_token",
//                     "lightstreamer_endpoint": "https://test.lightstreamer.com",
//                     "cst": "test_cst",
//                     "x_security_token": "test_x_security_token"
//                 }
//             "#,
//             )
//             .create_async()
//             .await;
//
//         let config = create_test_config(&server.url());
//         let mut session = Session::new(config).unwrap();
//
//         assert!(session.get_auth_tokens().is_none());
//
//         session.authenticate().await.unwrap();
//         let tokens = session.get_auth_tokens();
//         assert!(tokens.is_some());
//         let (cst, x_security_token) = tokens.unwrap();
//         assert_eq!(cst, "test_cst");
//         assert_eq!(x_security_token, "test_x_security_token");
//
//         mock.assert_async().await;
//     }
// }
