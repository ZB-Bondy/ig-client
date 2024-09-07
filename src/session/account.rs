/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 7/9/24
 ******************************************************************************/

/*

 */
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug,Serialize, Deserialize)]
pub(crate) struct Accounts {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "accountName")]
    pub account_name: String,
    pub preferred: bool,
    #[serde(rename = "accountType")]
    pub account_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AccountInfo {
    pub balance: f64,
    pub deposit: f64,
    #[serde(rename = "profitLoss")]
    pub profit_loss: f64,
    pub available: f64,
}

#[derive(Debug, Serialize)]
pub(crate) struct AccountSwitchRequest {
    #[serde(rename = "accountId")]
    pub(crate) account_id: String,
    #[serde(rename = "defaultAccount")]
    pub(crate) default_account: Option<bool>,
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

impl Default for AccountInfo {
    fn default() -> Self {
        AccountInfo {
            balance: 0.0,
            deposit: 0.0,
            profit_loss: 0.0,
            available: 0.0,
        }
    }
}

impl fmt::Display for Accounts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"accountId\":\"{}\",\"accountName\":\"{}\",\"preferred\":{},\"accountType\":\"{}\"}}",
               self.account_id, self.account_name, self.preferred, self.account_type)
    }
}

impl fmt::Display for AccountInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"balance\":{:.2},\"deposit\":{:.2},\"profitLoss\":{:.2},\"available\":{:.2}}}",
               self.balance, self.deposit, self.profit_loss, self.available)
    }
}

impl fmt::Display for AccountSwitchRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"accountId\":\"{}\",\"defaultAccount\":{}}}",
               self.account_id, self.default_account.unwrap_or(false))
    }
}

impl fmt::Display for AccountSwitchResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"dealingEnabled\":{},\"hasActiveDemoAccounts\":{},\"hasActiveLiveAccounts\":{},\"trailingStopsEnabled\":{}}}",
               self.dealing_enabled, self.has_active_demo_accounts, self.has_active_live_accounts, self.trailing_stops_enabled)
    }
}

#[cfg(test)]
mod tests_accounts {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn test_accounts_display() {
        let account = Accounts {
            account_id: "ABC123".to_string(),
            account_name: "Test Account".to_string(),
            preferred: true,
            account_type: "CFD".to_string(),
        };
        let display_output = account.to_string();
        let expected_json = json!({
            "accountId": "ABC123",
            "accountName": "Test Account",
            "preferred": true,
            "accountType": "CFD"
        });
        assert_json_diff::assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }
}

#[cfg(test)]
mod tests_account_info {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn test_account_info_display() {
        let info = AccountInfo {
            balance: 1000.50,
            deposit: 500.25,
            profit_loss: 200.75,
            available: 700.00,
        };
        let display_output = info.to_string();
        let expected_json = json!({
            "balance": 1000.50,
            "deposit": 500.25,
            "profitLoss": 200.75,
            "available": 700.00
        });
        assert_json_diff::assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }
}

#[cfg(test)]
mod tests_account_switch_request {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn test_account_switch_request_display() {
        let request = AccountSwitchRequest {
            account_id: "XYZ789".to_string(),
            default_account: Some(true),
        };
        let display_output = request.to_string();
        let expected_json = json!({
            "accountId": "XYZ789",
            "defaultAccount": true
        });
        assert_json_diff::assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }
}

#[cfg(test)]
mod tests_account_switch_response {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn test_account_switch_response_display() {
        let response = AccountSwitchResponse {
            dealing_enabled: true,
            has_active_demo_accounts: false,
            has_active_live_accounts: true,
            trailing_stops_enabled: false,
        };
        let display_output = response.to_string();
        let expected_json = json!({
            "dealingEnabled": true,
            "hasActiveDemoAccounts": false,
            "hasActiveLiveAccounts": true,
            "trailingStopsEnabled": false
        });
        assert_json_diff::assert_json_eq!(
            serde_json::from_str::<serde_json::Value>(&display_output).unwrap(),
            expected_json
        );
    }
}