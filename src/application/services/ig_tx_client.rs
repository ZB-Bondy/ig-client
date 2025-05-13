use std::str::FromStr;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use reqwest::{Client, StatusCode};
use regex::Regex;
use tracing::debug;
use crate::application::models::transaction::{RawTransaction, Transaction};
use crate::config::Config;
use crate::error::AppError;
use crate::session::interface::IgSession;

#[async_trait]
pub trait IgTxFetcher {
    async fn fetch_range(
        &self,
        sess: &IgSession,
        from: DateTime<Utc>,
        to:   DateTime<Utc>,
    ) -> Result<Vec<Transaction>, AppError>;
}

pub struct IgTxClient<'a> {
    cfg:   &'a Config,
    http:  Client,
    re:    Regex,
}

impl<'a> IgTxClient<'a> {
    pub fn new(cfg: &'a Config) -> Self {
        let re = Regex::new(
            r"(?P<under>[\p{L}0-9 ]+?)\s+(?P<strike>\d+(?:\.\d+)?)\s+(?P<kind>PUT|CALL)"
        ).unwrap();

        Self {
            cfg,
            http: Client::builder()
                .user_agent("ig-rs/0.1")
                .build()
                .expect("reqwest"),
            re,
        }
    }

    #[allow(dead_code)]
    fn rest_url(&self, path: &str) -> String {
        format!("{}/{}", self.cfg.rest_api.base_url.trim_end_matches('/'), path)
    }

    fn convert(&self, raw: RawTransaction) -> Transaction {
        // -------- regex -------------
        let caps = self.re.captures(&raw.instrument_name);

        let (underlying, strike, option_type) = if let Some(c) = caps.as_ref() {
            let under  = c.name("under").map(|m| m.as_str().trim().to_uppercase());
            let strike = c.name("strike")
                .and_then(|m| m.as_str().parse::<f64>().ok());
            let kind   = c.name("kind").map(|m| m.as_str().to_string());
            (under, strike, kind)
        } else {
            (None, None, None)
        };

        let deal_date = match chrono::NaiveDateTime::parse_from_str(&raw.date_utc, "%Y-%m-%dT%H:%M:%S") {
            Ok(naive) => Ok(naive.and_utc()),
            Err(e) => Err(e.into()), 
        };

        let pnl_eur = raw.pnl_raw.trim_start_matches('E')
            .parse::<f64>()
            .unwrap_or(0.0);

        let expiry = raw.period.split_once('-').and_then(|(mon, yy)| {
            chrono::Month::from_str(mon).ok()
                .and_then(|m| NaiveDate::from_ymd_opt(2000 + yy.parse::<i32>().ok()?, m.number_from_month(), 1))
        });

        let is_fee = raw.transaction_type == "WITH" && pnl_eur.abs() < 1.0;

        Transaction {
            deal_date,
            underlying,
            strike,
            option_type,
            expiry,
            transaction_type: raw.transaction_type.clone(),
            pnl_eur,
            reference: raw.reference.clone(),
            is_fee,
            raw_json: raw.to_string(),
        }
    }
}

#[async_trait]
impl<'a> IgTxFetcher for IgTxClient<'a> {
    async fn fetch_range(
        &self,
        sess: &IgSession,
        from: DateTime<Utc>,
        to:   DateTime<Utc>,
    ) -> Result<Vec<Transaction>, AppError> {

        let mut page = 1;
        let mut out  = Vec::new();

        loop {
            let url = format!(
                "{}/history/transactions?from={}&to={}&pageNumber={}&pageSize=200",
                self.cfg.rest_api.base_url,
                from.format("%Y-%m-%dT%H:%M:%S"),
                to  .format("%Y-%m-%dT%H:%M:%S"),
                page
            );
            debug!("🔗 Fetching IG txs from URL: {}", url);

            let resp = self.http
                .get(&url)
                .header("X-IG-API-KEY", &self.cfg.credentials.api_key)
                .header("CST",             &sess.cst)
                .header("X-SECURITY-TOKEN",&sess.token)
                .header("Version","2")
                .header("Accept","application/json; charset=UTF-8")
                .send()
                .await?;

            if resp.status() != StatusCode::OK {
                return Err(AppError::Unexpected(resp.status()));
            }

            let json: serde_json::Value = resp.json().await?;
            let raws: Vec<RawTransaction> =
                serde_json::from_value(json["transactions"].clone()).unwrap_or_default();

            if raws.is_empty() { break; }

            out.extend(raws.into_iter().map(|r| self.convert(r)));

            let meta = &json["metadata"]["pageData"];
            let total_pages = meta["totalPages"].as_u64().unwrap_or(1);
            if page >= total_pages { break; }
            page += 1;
        }

        Ok(out)
    }
}