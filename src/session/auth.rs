use crate::session::account::{AccountInfo, Accounts};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/*
Authentication and authorisation
There are currently two mechanisms for logging into and accessing the API.

POST /session v1 and v2 return a CST header with a token identifying a client and an X-SECURITY-TOKEN header with a token identifying the current account. These headers should be passed on subsequent requests to the API. Both tokens are initially valid for 6 hours but get extended up to a maximum of 72 hours while they are in use.

POST /session v3 returns OAuth access and refresh tokens which the user can pass in subsequent API requests via the Authorization header, e.g.:

Authorization : Bearer 5d1ea445-568b-4748-ab47-af9b982bfb74

The access token only identifies the client so users should also pass an IG-ACCOUNT-ID header to specify the account the request applies to, e.g.:
IG-ACCOUNT-ID : PZVI2

The access token is only valid for a limited period of time (e.g. 60 seconds) specified by the login response.

       "oauthToken": {
               "access_token": "702f6580-25c7-4c04-931d-6000efa824f8",
               "refresh_token": "a9cec2d7-fd01-4d16-a2dd-7427ef6a471d",
               "scope": "profile",
               "token_type": "Bearer",
               "expires_in": "60"
       }
The refresh token can used to acquire a new access token, either before or after the access token has expired but please note that the refresh token does also expiry some time after the access token has expired (e.g. 10 minutes). A call to refresh an access token will also return a new refresh token.The scope for individual clients is always profile which allows full access to the user's account.
 */

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

impl Default for AuthResponse {
    fn default() -> Self {
        AuthResponse {
            account_type: "".to_string(),
            account_info: AccountInfo::default(),
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
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
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

impl Default for AuthVersionResponse {
    fn default() -> Self {
        AuthVersionResponse::V1(AuthResponse::default())
    }
}

#[derive(Debug)]
pub(crate) struct AuthInfo {
    pub(crate) auth_response: AuthVersionResponse,
    pub(crate) expires_at: DateTime<Utc>,
    pub(crate) cst: Option<String>,
    pub(crate) x_security_token: Option<String>,
    pub(crate) oauth_token: Option<OAuthToken>,
}

impl AuthInfo {
    pub fn new(
        auth_response: AuthVersionResponse,
        expires_at: DateTime<Utc>,
        cst: Option<String>,
        x_security_token: Option<String>,
        oauth_token: Option<OAuthToken>,
    ) -> Self {
        Self {
            auth_response,
            expires_at,
            cst,
            x_security_token,
            oauth_token,
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

    #[test]
    fn test_auth_info_display() {
        // Crear un tiempo fijo: 2023-09-07T12:00:00Z
        let expires_at = Utc::now();

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
            None,
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
