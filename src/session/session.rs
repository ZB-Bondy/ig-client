/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 7/9/24
 ******************************************************************************/
use std::time::{Duration, Instant};
use anyhow::Context;
use tracing::{debug, error, instrument};
use crate::config::Config;
use crate::session::account::{AccountSwitchRequest, AccountSwitchResponse};
use crate::session::auth::{AuthInfo, AuthRequest, AuthResponse, AuthResponseV3, AuthVersionResponse};
use crate::session::auth::AuthVersionResponse::{V1, V3};
use crate::session::session_response::SessionResponse;
use crate::transport::http_client::IGHttpClient;

#[derive(Debug)]
pub struct Session {
    client: IGHttpClient,
    config: Config,
    auth_info: Option<AuthInfo>,
}

impl Session {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let client = IGHttpClient::new(&config.rest_api.base_url, &config.credentials.api_key)?;
        Ok(Self {
            client,
            config,
            auth_info: None,
        })
    }

    #[instrument(skip(self))]
    pub async fn authenticate(&mut self, version: u8) -> anyhow::Result<()> {
        let version_header = ("version".to_string(), version.to_string());
        let token_header = (
            "x-ig-api-key".to_string(),
            self.config.credentials.api_key.clone(),
        );
        let headers = vec![version_header, token_header];
        debug!("Headers: {:?}", headers);

        let (cst, x_security_token): (Option<String>, Option<String>) = (None, None);
        let auth_version_response: AuthVersionResponse =  match version {
            1 | 2 => {
                let auth_request = AuthRequest::new(
                    self.config.credentials.username.clone(),
                    self.config.credentials.password.clone(),
                    Some(false),
                );
                let (response, cst, x_security_token) = self
                    .client
                    .post_with_headers::<AuthResponse, AuthRequest>("/session", &auth_request, &headers)
                    .await
                    .context("Failed to authenticate")?;
                debug!("Authentication response v{}: {:?}", version, response);
                V1(response)
            }
            3 => {
                let auth_request = AuthRequest::new(
                    self.config.credentials.username.clone(),
                    self.config.credentials.password.clone(),
                    None,
                );
                let (response, cst, x_security_token) = self
                    .client
                    .post_with_headers::<AuthResponseV3, AuthRequest>("/session", &auth_request, &headers)
                    .await
                    .context("Failed to authenticate")?;
                debug!("Authentication response v{}: {:?}", version, response);
                V3(response)
            }
            _ => {
                panic!("Unsupported authentication version")
            }
        };

        debug!("Authenticating user: {}", self.config.credentials.username);
        let auth_info : Option<AuthInfo> = Some(AuthInfo::new(
            auth_version_response,
            Instant::now() + Duration::from_secs(60),
            cst,
            x_security_token));

        self.auth_info = auth_info;

        debug!("Authentication successful");
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn ensure_auth(&mut self) -> anyhow::Result<()> {
        if let Some(auth_info) = &self.auth_info {
            if auth_info.expires_at > Instant::now() {
                return Ok(());
            }
        }

        // TODO: If we reach here, we need to reauthenticate or refresh the token
        self.authenticate(1).await // Default to v1 authentication
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

    pub async fn refresh_token(&mut self) -> anyhow::Result<()> {
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

    pub async fn logout(&mut self) -> anyhow::Result<()> {
        self.client
            .delete::<()>("/session")
            .await
            .context("Failed to logout")?;
        self.auth_info = None;
        Ok(())
    }

    pub async fn switch_account( // TODO: Refactor to use /session endpoint with PUT
        &mut self,
        account_id: &str,
        set_default: Option<bool>,
    ) -> anyhow::Result<AccountSwitchResponse> {
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
            // auth_info.auth_response.current_account_id= account_id.to_string();
        }

        Ok(response)
    }

    pub async fn get_session_details(&self, fetch_session_tokens: bool) -> anyhow::Result<SessionResponse> {
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

        // let result = session.ensure_auth().await;

        // assert!(result.is_ok());
        // assert!(session.auth_info.is_some());
        //
        // mock.assert_async().await;
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

        // session.authenticate(3).await.unwrap();
        // let result = session.ensure_auth().await;
        // assert!(result.is_ok());
        //
        // mock.assert_async().await;
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

        // session.authenticate(3).await.unwrap();
        // tokio::time::sleep(Duration::from_secs(2)).await;
        // let result = session.ensure_auth().await;
        // assert!(result.is_ok());
        // mock.assert_async().await;
    }
}
