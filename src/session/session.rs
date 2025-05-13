#[derive(serde::Deserialize)]
pub struct SessionResp {
    #[serde(alias = "accountId")]
    #[serde(alias = "currentAccountId")]
    pub account_id: String,

    #[serde(alias = "clientId")]
    pub client_id: Option<String>,
    #[serde(alias = "timezoneOffset")]
    pub timezone_offset: Option<i32>,
}