/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 7/9/24
 ******************************************************************************/
use serde::Deserialize;

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

