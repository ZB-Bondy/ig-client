/******************************************************************************
   Author: Joaquín Béjar García
   Email: jb@taunais.com
   Date: 13/5/25
******************************************************************************/
use serde::{Deserialize, Serialize};

use super::order::Direction;

/// Información de la cuenta
#[derive(Debug, Clone, Deserialize)]
pub struct AccountInfo {
    pub accounts: Vec<Account>,
}

/// Detalles de una cuenta específica
#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "accountName")]
    pub account_name: String,
    #[serde(rename = "accountType")]
    pub account_type: String,
    pub balance: AccountBalance,
    pub currency: String,
    pub status: String,
    pub preferred: bool,
}

/// Balance de la cuenta
#[derive(Debug, Clone, Deserialize)]
pub struct AccountBalance {
    pub balance: f64,
    pub deposit: f64,
    #[serde(rename = "profitLoss")]
    pub profit_loss: f64,
    pub available: f64,
}

/// Actividad de la cuenta
#[derive(Debug, Clone, Deserialize)]
pub struct AccountActivity {
    pub activities: Vec<Activity>,
}

/// Actividad individual
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Activity {
    pub date: String,
    #[serde(rename = "dealId")]
    pub deal_id: String,
    pub epic: String,
    pub period: String,
    #[serde(rename = "dealReference")]
    pub deal_reference: String,
    #[serde(rename = "activityType")]
    pub activity_type: String,
    pub status: String,
    pub description: String,
    pub details: Option<String>,
}

/// Posiciones abiertas
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Positions {
    pub positions: Vec<Position>,
}

/// Posición individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub position: PositionDetails,
    pub market: PositionMarket,
    pub pnl: Option<f64>,
}

/// Details of a position
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PositionDetails {
    #[serde(rename = "contractSize")]
    pub contract_size: f64,
    #[serde(rename = "createdDate")]
    pub created_date: String,
    #[serde(rename = "createdDateUTC")]
    pub created_date_utc: String,
    #[serde(rename = "dealId")]
    pub deal_id: String,
    #[serde(rename = "dealReference")]
    pub deal_reference: String,
    pub direction: Direction,
    #[serde(rename = "limitLevel")]
    pub limit_level: Option<f64>,
    pub level: f64,
    pub size: f64,
    #[serde(rename = "stopLevel")]
    pub stop_level: Option<f64>,
    #[serde(rename = "trailingStep")]
    pub trailing_step: Option<f64>,
    #[serde(rename = "trailingStopDistance")]
    pub trailing_stop_distance: Option<f64>,
    pub currency: String,
    #[serde(rename = "controlledRisk")]
    pub controlled_risk: bool,
    #[serde(rename = "limitedRiskPremium")]
    pub limited_risk_premium: Option<f64>,
}

/// Market information for a position
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PositionMarket {
    #[serde(rename = "instrumentName")]
    pub instrument_name: String,
    pub expiry: String,
    pub epic: String,
    #[serde(rename = "instrumentType")]
    pub instrument_type: String,
    #[serde(rename = "lotSize")]
    pub lot_size: f64,
    pub high: f64,
    pub low: f64,
    #[serde(rename = "percentageChange")]
    pub percentage_change: f64,
    #[serde(rename = "netChange")]
    pub net_change: f64,
    pub bid: f64,
    pub offer: f64,
    #[serde(rename = "updateTime")]
    pub update_time: String,
    #[serde(rename = "updateTimeUTC")]
    pub update_time_utc: String,
    #[serde(rename = "delayTime")]
    pub delay_time: i64,
    #[serde(rename = "streamingPricesAvailable")]
    pub streaming_prices_available: bool,
    #[serde(rename = "marketStatus")]
    pub market_status: String,
    #[serde(rename = "scalingFactor")]
    pub scaling_factor: i64
}

/// Órdenes de trabajo
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkingOrders {
    #[serde(rename = "workingOrders")]
    pub working_orders: Vec<WorkingOrder>,
}

/// Working order
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkingOrder {
    #[serde(rename = "workingOrderData")]
    pub working_order_data: WorkingOrderData,
    #[serde(rename = "marketData")]
    pub market_data: MarketData,
}

/// Details of a working order
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkingOrderData {
    #[serde(rename = "dealId")]
    pub deal_id: String,
    pub direction: Direction,
    pub epic: String,
    #[serde(rename = "orderSize")]
    pub order_size: f64,
    #[serde(rename = "orderLevel")]
    pub order_level: f64,
    #[serde(rename = "timeInForce")]
    pub time_in_force: String,
    #[serde(rename = "goodTillDate")]
    pub good_till_date: Option<String>,
    #[serde(rename = "goodTillDateISO")]
    pub good_till_date_iso: Option<String>,
    #[serde(rename = "createdDate")]
    pub created_date: String,
    #[serde(rename = "createdDateUTC")]
    pub created_date_utc: String,
    #[serde(rename = "guaranteedStop")]
    pub guaranteed_stop: bool,
    #[serde(rename = "orderType")]
    pub order_type: String,
    #[serde(rename = "stopDistance")]
    pub stop_distance: Option<f64>,
    #[serde(rename = "limitDistance")]
    pub limit_distance: Option<f64>,
    #[serde(rename = "currencyCode")]
    pub currency_code: String,
    pub dma: bool,
    #[serde(rename = "limitedRiskPremium")]
    pub limited_risk_premium: Option<f64>,
    // Optional fields that might be present in other responses
    #[serde(rename = "limitLevel", default)]
    pub limit_level: Option<f64>,
    #[serde(rename = "stopLevel", default)]
    pub stop_level: Option<f64>,
    #[serde(rename = "dealReference", default)]
    pub deal_reference: Option<String>,
}

/// Market data for a working order
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketData {
    #[serde(rename = "instrumentName")]
    pub instrument_name: String,
    #[serde(rename = "exchangeId")]
    pub exchange_id: String,
    pub expiry: String,
    #[serde(rename = "marketStatus")]
    pub market_status: String,
    pub epic: String,
    #[serde(rename = "instrumentType")]
    pub instrument_type: String,
    #[serde(rename = "lotSize")]
    pub lot_size: f64,
    pub high: f64,
    pub low: f64,
    #[serde(rename = "percentageChange")]
    pub percentage_change: f64,
    #[serde(rename = "netChange")]
    pub net_change: f64,
    pub bid: f64,
    pub offer: f64,
    #[serde(rename = "updateTime")]
    pub update_time: String,
    #[serde(rename = "updateTimeUTC")]
    pub update_time_utc: String,
    #[serde(rename = "delayTime")]
    pub delay_time: i64,
    #[serde(rename = "streamingPricesAvailable")]
    pub streaming_prices_available: bool,
    #[serde(rename = "scalingFactor")]
    pub scaling_factor: i64,
}

/// Historial de transacciones
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionHistory {
    pub transactions: Vec<Transaction>,
    pub metadata: TransactionMetadata,
}

/// Metadatos de transacciones
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionMetadata {
    #[serde(rename = "pageData")]
    pub page_data: PageData,
    pub size: i32,
}

/// Información de paginación
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PageData {
    #[serde(rename = "pageNumber")]
    pub page_number: i32,
    #[serde(rename = "pageSize")]
    pub page_size: i32,
    #[serde(rename = "totalPages")]
    pub total_pages: i32,
}

/// Transacción individual
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Transaction {
    pub date: String,
    #[serde(rename = "dateUtc")]
    pub date_utc: String,
    #[serde(rename = "instrumentName")]
    pub instrument_name: String,
    pub period: String,
    #[serde(rename = "profitAndLoss")]
    pub profit_and_loss: String,
    #[serde(rename = "transactionType")]
    pub transaction_type: String,
    pub reference: String,
    #[serde(rename = "openLevel")]
    pub open_level: String,
    #[serde(rename = "closeLevel")]
    pub close_level: String,
    pub size: String,
    pub currency: String,
    #[serde(rename = "cashTransaction")]
    pub cash_transaction: bool,
}
