/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 13/5/25
 ******************************************************************************/
use serde::{Deserialize, Serialize};

/// Dirección de la orden (compra o venta)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Direction {
    Buy,
    Sell,
}

/// Tipo de orden
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    Limit,
    Market,
    Quote,
    Stop,
    StopLimit,
}

/// Estado de la orden
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderStatus {
    Accepted,
    Rejected,
    Working,
    Filled,
    Cancelled,
    Expired,
}

/// Duración de la orden
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimeInForce {
    #[serde(rename = "GOOD_TILL_CANCELLED")]
    GoodTillCancelled,
    #[serde(rename = "GOOD_TILL_DATE")]
    GoodTillDate,
    #[serde(rename = "IMMEDIATE_OR_CANCEL")]
    ImmediateOrCancel,
    #[serde(rename = "FILL_OR_KILL")]
    FillOrKill,
}

/// Modelo para crear una nueva orden
#[derive(Debug, Clone, Serialize)]
pub struct CreateOrderRequest {
    pub epic: String,
    pub direction: Direction,
    pub size: f64,
    #[serde(rename = "orderType")]
    pub order_type: OrderType,
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,
    #[serde(rename = "level", skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    #[serde(rename = "guaranteedStop", skip_serializing_if = "Option::is_none")]
    pub guaranteed_stop: Option<bool>,
    #[serde(rename = "stopLevel", skip_serializing_if = "Option::is_none")]
    pub stop_level: Option<f64>,
    #[serde(rename = "stopDistance", skip_serializing_if = "Option::is_none")]
    pub stop_distance: Option<f64>,
    #[serde(rename = "limitLevel", skip_serializing_if = "Option::is_none")]
    pub limit_level: Option<f64>,
    #[serde(rename = "limitDistance", skip_serializing_if = "Option::is_none")]
    pub limit_distance: Option<f64>,
    #[serde(rename = "expiry", skip_serializing_if = "Option::is_none")]
    pub expiry: Option<String>,
    #[serde(rename = "dealReference", skip_serializing_if = "Option::is_none")]
    pub deal_reference: Option<String>,
    #[serde(rename = "forceOpen", skip_serializing_if = "Option::is_none")]
    pub force_open: Option<bool>,
}

impl CreateOrderRequest {
    /// Crea una nueva orden de mercado
    pub fn market(epic: String, direction: Direction, size: f64) -> Self {
        Self {
            epic,
            direction,
            size,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::FillOrKill,
            level: None,
            guaranteed_stop: None,
            stop_level: None,
            stop_distance: None,
            limit_level: None,
            limit_distance: None,
            expiry: None,
            deal_reference: None,
            force_open: Some(true),
        }
    }

    /// Crea una nueva orden limitada
    pub fn limit(epic: String, direction: Direction, size: f64, level: f64) -> Self {
        Self {
            epic,
            direction,
            size,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GoodTillCancelled,
            level: Some(level),
            guaranteed_stop: None,
            stop_level: None,
            stop_distance: None,
            limit_level: None,
            limit_distance: None,
            expiry: None,
            deal_reference: None,
            force_open: Some(true),
        }
    }

    /// Añade un stop loss a la orden
    pub fn with_stop_loss(mut self, stop_level: f64) -> Self {
        self.stop_level = Some(stop_level);
        self
    }

    /// Añade un take profit a la orden
    pub fn with_take_profit(mut self, limit_level: f64) -> Self {
        self.limit_level = Some(limit_level);
        self
    }

    /// Añade una referencia a la orden
    pub fn with_reference(mut self, reference: String) -> Self {
        self.deal_reference = Some(reference);
        self
    }
}

/// Respuesta a la creación de una orden
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrderResponse {
    #[serde(rename = "dealReference")]
    pub deal_reference: String,
}

/// Detalles de una orden confirmada
#[derive(Debug, Clone, Deserialize)]
pub struct OrderConfirmation {
    pub date: String,
    pub status: OrderStatus,
    pub reason: Option<String>,
    #[serde(rename = "dealId")]
    pub deal_id: Option<String>,
    #[serde(rename = "dealReference")]
    pub deal_reference: String,
    #[serde(rename = "dealStatus")]
    pub deal_status: Option<String>,
    pub epic: Option<String>,
    #[serde(rename = "expiry")]
    pub expiry: Option<String>,
    #[serde(rename = "guaranteedStop")]
    pub guaranteed_stop: Option<bool>,
    #[serde(rename = "level")]
    pub level: Option<f64>,
    #[serde(rename = "limitDistance")]
    pub limit_distance: Option<f64>,
    #[serde(rename = "limitLevel")]
    pub limit_level: Option<f64>,
    pub size: Option<f64>,
    #[serde(rename = "stopDistance")]
    pub stop_distance: Option<f64>,
    #[serde(rename = "stopLevel")]
    pub stop_level: Option<f64>,
    #[serde(rename = "trailingStop")]
    pub trailing_stop: Option<bool>,
    pub direction: Option<Direction>,
}

/// Modelo para modificar una posición existente
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePositionRequest {
    #[serde(rename = "stopLevel", skip_serializing_if = "Option::is_none")]
    pub stop_level: Option<f64>,
    #[serde(rename = "limitLevel", skip_serializing_if = "Option::is_none")]
    pub limit_level: Option<f64>,
    #[serde(rename = "trailingStop", skip_serializing_if = "Option::is_none")]
    pub trailing_stop: Option<bool>,
    #[serde(rename = "trailingStopDistance", skip_serializing_if = "Option::is_none")]
    pub trailing_stop_distance: Option<f64>,
}

/// Modelo para cerrar una posición existente
#[derive(Debug, Clone, Serialize)]
pub struct ClosePositionRequest {
    #[serde(rename = "dealId")]
    pub deal_id: String,
    pub direction: Direction,
    pub size: f64,
    #[serde(rename = "orderType")]
    pub order_type: OrderType,
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,
    #[serde(rename = "level", skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
}

impl ClosePositionRequest {
    /// Crea una solicitud para cerrar una posición al mercado
    pub fn market(deal_id: String, direction: Direction, size: f64) -> Self {
        Self {
            deal_id,
            direction,
            size,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::FillOrKill,
            level: None,
        }
    }
}

/// Respuesta al cerrar una posición
#[derive(Debug, Clone, Deserialize)]
pub struct ClosePositionResponse {
    #[serde(rename = "dealReference")]
    pub deal_reference: String,
}
