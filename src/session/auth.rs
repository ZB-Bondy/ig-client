use crate::session::account::{AccountInfo, Accounts};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Instant;

#[derive(Debug, Serialize)]
pub(crate) struct AuthRequest {
    identifier: String,
    password: String,
    #[serde(rename = "encryptedPassword")]
    encrypted_password: Option<bool>, // Used in version 1 and 2
}

#[derive(Debug, Deserialize)]
pub struct AuthResponseV3 {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "lightstreamerEndpoint")]
    pub lightstreamer_endpoint: String,
    #[serde(rename = "oauthToken")]
    pub oauth_token: Option<OAuthToken>,
    #[serde(rename = "timezoneOffset")]
    pub timezone_offset: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AuthResponse {
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

#[derive(Debug)]
pub(crate) enum AuthVersionResponse {
    V1(AuthResponse),
    V2(AuthResponse),
    V3(AuthResponseV3),
}

#[derive(Debug)]
pub(crate) struct AuthInfo {
    pub(crate) auth_response: AuthVersionResponse,
    pub(crate) expires_at: Instant,
    cst: Option<String>,
    x_security_token: Option<String>,
}

impl AuthInfo {
    pub fn new(
        auth_response: AuthVersionResponse,
        expires_at: Instant,
        cst: Option<String>,
        x_security_token: Option<String>,
    ) -> Self {
        Self {
            auth_response,
            expires_at,
            cst,
            x_security_token,
        }
    }
}

impl AuthRequest {
    pub fn new(identifier: String, password: String, encrypted_password: Option<bool>) -> Self {
        Self {
            identifier,
            password,
            encrypted_password,
        }
    }
}

impl fmt::Display for AuthRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"identifier\":\"{}\",\"password\":\"[REDACTED]\",\"encryptedPassword\":{}}}",
            self.identifier,
            self.encrypted_password.unwrap_or(false)
        )
    }
}

impl fmt::Display for AuthResponseV3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"accountId\":\"{}\",\"clientId\":\"{}\",\"lightstreamerEndpoint\":\"{}\",\"oauthToken\":{},\"timezoneOffset\":{}}}",
               self.account_id, self.client_id, self.lightstreamer_endpoint,
               self.oauth_token.as_ref().map_or("null".to_string(), |t| t.to_string()),
               self.timezone_offset)
    }
}

impl fmt::Display for AuthResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"accountType\":\"{}\",\"accountInfo\":{},\"currencyIsoCode\":\"{}\",\"currencySymbol\":\"{}\",\"currentAccountId\":\"{}\",\"lightstreamerEndpoint\":\"{}\",\"accounts\":[{}],\"clientId\":\"{}\",\"timezoneOffset\":{},\"hasActiveDemoAccounts\":{},\"hasActiveLiveAccounts\":{},\"trailingStopsEnabled\":{},\"reroutingEnvironment\":{},\"dealingEnabled\":{}}}",
               self.account_type, self.account_info, self.currency_iso_code, self.currency_symbol,
               self.current_account_id, self.lightstreamer_endpoint,
               self.accounts.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(","),
               self.client_id, self.timezone_offset, self.has_active_demo_accounts,
               self.has_active_live_accounts, self.trailing_stops_enabled,
               self.rerouting_environment.as_ref().map_or("null".to_string(), |s| format!("\"{}\"", s)),
               self.dealing_enabled)
    }
}

impl fmt::Display for OAuthToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"access_token\":\"[REDACTED]\",\"refresh_token\":\"[REDACTED]\",\"scope\":\"{}\",\"token_type\":\"{}\",\"expires_in\":\"{}\"}}",
               self.scope, self.token_type, self.expires_in)
    }
}

impl fmt::Display for AuthVersionResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthVersionResponse::V1(response) => write!(f, "{}", response),
            AuthVersionResponse::V2(response) => write!(f, "{}", response),
            AuthVersionResponse::V3(response) => write!(f, "{}", response),
        }
    }
}

impl fmt::Display for AuthInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"auth_response\":{},\"expires_at\":\"{:?}\",\"cst\":\"[REDACTED]\",\"x_security_token\":\"[REDACTED]\"}}",
               self.auth_response, self.expires_at)
    }
}

#[cfg(test)]
mod tests_auth_request {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_auth_request_display() {
        let request =
            AuthRequest::new("user123".to_string(), "password123".to_string(), Some(true));
        let display_output = request.to_string();
        let expected_json = json!({
            "identifier": "user123",
            "password": "[REDACTED]",
            "encryptedPassword": true
        });
        assert_json_diff::assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }
}

#[cfg(test)]
mod tests_auth_response_v3 {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_auth_response_v3_display() {
        let response = AuthResponseV3 {
            account_id: "ACC123".to_string(),
            client_id: "CLIENT456".to_string(),
            lightstreamer_endpoint: "wss://example.com".to_string(),
            oauth_token: Some(OAuthToken {
                access_token: "access123".to_string(),
                refresh_token: "refresh456".to_string(),
                scope: "scope789".to_string(),
                token_type: "Bearer".to_string(),
                expires_in: "3600".to_string(),
            }),
            timezone_offset: 1.0,
        };
        let display_output = response.to_string();
        let expected_json = json!({
            "accountId": "ACC123",
            "clientId": "CLIENT456",
            "lightstreamerEndpoint": "wss://example.com",
            "oauthToken": {
                "access_token": "[REDACTED]",
                "refresh_token": "[REDACTED]",
                "scope": "scope789",
                "token_type": "Bearer",
                "expires_in": "3600"
            },
            "timezoneOffset": 1
        });
        assert_json_diff::assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }
}

#[cfg(test)]
mod tests_auth_response {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_auth_response_display() {
        let response = AuthResponse {
            account_type: "CFD".to_string(),
            account_info: AccountInfo {
                balance: 1000.0,
                deposit: 500.0,
                profit_loss: 200.0,
                available: 700.0,
            },
            currency_iso_code: "USD".to_string(),
            currency_symbol: "$".to_string(),
            current_account_id: "ACC789".to_string(),
            lightstreamer_endpoint: "wss://example.com".to_string(),
            accounts: vec![Accounts {
                account_id: "ACC789".to_string(),
                account_name: "Main Account".to_string(),
                preferred: true,
                account_type: "CFD".to_string(),
            }],
            client_id: "CLIENT123".to_string(),
            timezone_offset: -5,
            has_active_demo_accounts: false,
            has_active_live_accounts: true,
            trailing_stops_enabled: true,
            rerouting_environment: Some("LIVE".to_string()),
            dealing_enabled: true,
        };
        let display_output = response.to_string();
        let expected_json = json!({
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
        assert_json_diff::assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }
}

#[cfg(test)]
mod tests_oauth_token {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_oauth_token_display() {
        let token = OAuthToken {
            access_token: "access123".to_string(),
            refresh_token: "refresh456".to_string(),
            scope: "scope789".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: "3600".to_string(),
        };
        let display_output = token.to_string();
        let expected_json = json!({
            "access_token": "[REDACTED]",
            "refresh_token": "[REDACTED]",
            "scope": "scope789",
            "token_type": "Bearer",
            "expires_in": "3600"
        });
        assert_json_diff::assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }
}

#[cfg(test)]
mod tests_auth_info {
    use super::*;
    use serde_json::json;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn test_auth_info_display() {
        // Crear un tiempo fijo: 2023-09-07T12:00:00Z
        let fixed_time = UNIX_EPOCH + Duration::from_secs(1694088000);
        let expires_at = Instant::now()
            .checked_add(fixed_time.duration_since(UNIX_EPOCH).unwrap())
            .unwrap();

        let auth_response = AuthResponse {
            account_type: "CFD".to_string(),
            account_info: AccountInfo {
                balance: 1000.0,
                deposit: 500.0,
                profit_loss: 200.0,
                available: 700.0,
            },
            currency_iso_code: "USD".to_string(),
            currency_symbol: "$".to_string(),
            current_account_id: "ACC789".to_string(),
            lightstreamer_endpoint: "wss://example.com".to_string(),
            accounts: vec![Accounts {
                account_id: "ACC789".to_string(),
                account_name: "Main Account".to_string(),
                preferred: true,
                account_type: "CFD".to_string(),
            }],
            client_id: "CLIENT123".to_string(),
            timezone_offset: -5,
            has_active_demo_accounts: false,
            has_active_live_accounts: true,
            trailing_stops_enabled: true,
            rerouting_environment: Some("LIVE".to_string()),
            dealing_enabled: true,
        };

        let auth_info = AuthInfo::new(
            AuthVersionResponse::V1(auth_response),
            expires_at,
            Some("cst123".to_string()),
            Some("token456".to_string()),
        );

        let display_output = auth_info.to_string();
        let expected_json = json!({
            "auth_response": {
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
            },
            "expires_at": format!("{:?}", expires_at),
            "cst": "[REDACTED]",
            "x_security_token": "[REDACTED]"
        });

        assert_json_diff::assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }
}
