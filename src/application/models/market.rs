/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 13/5/25
 ******************************************************************************/
use serde::{Deserialize, Serialize};

/// Tipo de instrumento
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstrumentType {
    Shares,
    Currencies,
    Indices,
    SprintMarket,
    Commodities,
    Options,
    #[serde(rename = "BINARY")]
    Binary,
    #[serde(other)]
    Unknown,
}

/// Modelo para un instrumento de mercado
#[derive(Debug, Clone, Deserialize)]
pub struct Instrument {
    pub epic: String,
    pub name: String,
    #[serde(rename = "instrumentType")]
    pub instrument_type: InstrumentType,
    pub expiry: String,
    #[serde(rename = "contractSize")]
    pub contract_size: Option<f64>,
    #[serde(rename = "lotSize")]
    pub lot_size: Option<f64>,
    #[serde(rename = "highLimitPrice")]
    pub high_limit_price: Option<f64>,
    #[serde(rename = "lowLimitPrice")]
    pub low_limit_price: Option<f64>,
    #[serde(rename = "marginFactor")]
    pub margin_factor: Option<f64>,
    #[serde(rename = "marginFactorUnit")]
    pub margin_factor_unit: Option<String>,
    #[serde(rename = "slippageFactor")]
    pub slippage_factor: Option<f64>,
    #[serde(rename = "limitedRiskPremium")]
    pub limited_risk_premium: Option<f64>,
    #[serde(rename = "newsCode")]
    pub news_code: Option<String>,
    #[serde(rename = "chartCode")]
    pub chart_code: Option<String>,
    pub currencies: Option<Vec<Currency>>,
}

/// Modelo para la divisa de un instrumento
#[derive(Debug, Clone, Deserialize)]
pub struct Currency {
    pub code: String,
    pub symbol: Option<String>,
    #[serde(rename = "baseExchangeRate")]
    pub base_exchange_rate: Option<f64>,
    #[serde(rename = "exchangeRate")]
    pub exchange_rate: Option<f64>,
    #[serde(rename = "isDefault")]
    pub is_default: Option<bool>,
}

/// Modelo para los datos de mercado
#[derive(Debug, Clone, Deserialize)]
pub struct MarketDetails {
    pub instrument: Instrument,
    pub snapshot: MarketSnapshot,
}

/// Reglas de negociación para un mercado
#[derive(Debug, Clone, Deserialize)]
pub struct DealingRules {
    #[serde(rename = "minDealSize")]
    pub min_deal_size: Option<f64>,
    #[serde(rename = "maxDealSize")]
    pub max_deal_size: Option<f64>,
    #[serde(rename = "minControlledRiskStopDistance")]
    pub min_controlled_risk_stop_distance: Option<f64>,
    #[serde(rename = "minNormalStopOrLimitDistance")]
    pub min_normal_stop_or_limit_distance: Option<f64>,
    #[serde(rename = "maxStopOrLimitDistance")]
    pub max_stop_or_limit_distance: Option<f64>,
    #[serde(rename = "marketOrderPreference")]
    pub market_order_preference: String,
    #[serde(rename = "trailingStopsPreference")]
    pub trailing_stops_preference: String,
}

/// Instantánea de mercado
#[derive(Debug, Clone, Deserialize)]
pub struct MarketSnapshot {
    #[serde(rename = "marketStatus")]
    pub market_status: String,
    #[serde(rename = "netChange")]
    pub net_change: Option<f64>,
    #[serde(rename = "percentageChange")]
    pub percentage_change: Option<f64>,
    #[serde(rename = "updateTime")]
    pub update_time: Option<String>,
    #[serde(rename = "delayTime")]
    pub delay_time: Option<i64>,
    pub bid: Option<f64>,
    pub offer: Option<f64>,
    #[serde(rename = "high")]
    pub high: Option<f64>,
    #[serde(rename = "low")]
    pub low: Option<f64>,
    #[serde(rename = "binaryOdds")]
    pub binary_odds: Option<f64>,
    #[serde(rename = "decimalPlacesFactor")]
    pub decimal_places_factor: Option<i64>,
    #[serde(rename = "scalingFactor")]
    pub scaling_factor: Option<i64>,
    #[serde(rename = "controlledRiskExtraSpread")]
    pub controlled_risk_extra_spread: Option<f64>,
}

/// Modelo para la búsqueda de mercados
#[derive(Debug, Clone, Deserialize)]
pub struct MarketSearchResult {
    pub markets: Vec<MarketData>,
}

/// Datos básicos de un mercado
#[derive(Debug, Clone, Deserialize)]
pub struct MarketData {
    pub epic: String,
    #[serde(rename = "instrumentName")]
    pub instrument_name: String,
    #[serde(rename = "instrumentType")]
    pub instrument_type: InstrumentType,
    pub expiry: String,
    #[serde(rename = "highLimitPrice")]
    pub high_limit_price: Option<f64>,
    #[serde(rename = "lowLimitPrice")]
    pub low_limit_price: Option<f64>,
    #[serde(rename = "marketStatus")]
    pub market_status: String,
    #[serde(rename = "netChange")]
    pub net_change: Option<f64>,
    #[serde(rename = "percentageChange")]
    pub percentage_change: Option<f64>,
    #[serde(rename = "updateTime")]
    pub update_time: Option<String>,
    pub bid: Option<f64>,
    pub offer: Option<f64>,
}

/// Modelo para los precios históricos
#[derive(Debug, Clone, Deserialize)]
pub struct HistoricalPricesResponse {
    pub prices: Vec<HistoricalPrice>,
    #[serde(rename = "instrumentType")]
    pub instrument_type: InstrumentType,
    #[serde(rename = "allowance")]
    pub allowance: PriceAllowance,
}

/// Precio histórico
#[derive(Debug, Clone, Deserialize)]
pub struct HistoricalPrice {
    #[serde(rename = "snapshotTime")]
    pub snapshot_time: String,
    #[serde(rename = "openPrice")]
    pub open_price: PricePoint,
    #[serde(rename = "highPrice")]
    pub high_price: PricePoint,
    #[serde(rename = "lowPrice")]
    pub low_price: PricePoint,
    #[serde(rename = "closePrice")]
    pub close_price: PricePoint,
    #[serde(rename = "lastTradedVolume")]
    pub last_traded_volume: Option<i64>,
}

/// Punto de precio
#[derive(Debug, Clone, Deserialize)]
pub struct PricePoint {
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    #[serde(rename = "lastTraded")]
    pub last_traded: Option<f64>,
}

/// Información sobre la asignación de precios
#[derive(Debug, Clone, Deserialize)]
pub struct PriceAllowance {
    #[serde(rename = "remainingAllowance")]
    pub remaining_allowance: i64,
    #[serde(rename = "totalAllowance")]
    pub total_allowance: i64,
    #[serde(rename = "allowanceExpiry")]
    pub allowance_expiry: i64,
}
