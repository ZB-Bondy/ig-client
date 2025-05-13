/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 12/5/25
 ******************************************************************************/
use std::fmt;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Raw JSON coming from IG’s transactions endpoint
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawTransaction {
    #[serde(rename = "date")]
    pub(crate) date: String,

    #[serde(rename = "dateUtc")]
    pub(crate) date_utc: String,

    #[serde(rename = "openDateUtc")]
    pub(crate) open_date_utc: String,

    #[serde(rename = "instrumentName")]
    pub(crate) instrument_name: String,

    #[serde(rename = "period")]
    pub(crate) period: String,

    #[serde(rename = "profitAndLoss")]
    pub(crate) pnl_raw: String,

    #[serde(rename = "transactionType")]
    pub(crate) transaction_type: String,

    pub(crate) reference: String,

    #[serde(rename = "openLevel")]
    pub(crate) open_level: String,

    #[serde(rename = "closeLevel")]
    pub(crate) close_level: String,

    #[serde(rename = "size")]
    pub(crate) size: String,

    #[serde(rename = "currency")]
    pub(crate) currency: String,

    #[serde(rename = "cashTransaction")]
    pub(crate) cash_transaction: bool,
}

impl fmt::Display for RawTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_json::to_string(self)
            .map_err(|_| fmt::Error)?;
        write!(f, "{}", s)
    }
}

#[derive(Debug)]
pub struct Transaction {
    pub(crate) deal_date: DateTime<Utc>,
    pub(crate) underlying: Option<String>,
    pub(crate) strike: Option<f64>,
    pub(crate) option_type: Option<String>,
    pub(crate) expiry: Option<NaiveDate>,
    pub(crate) transaction_type: String,
    pub(crate) pnl_eur: f64,
    pub(crate) reference: String,
    pub(crate) is_fee: bool,
    pub(crate) raw_json: String,
}