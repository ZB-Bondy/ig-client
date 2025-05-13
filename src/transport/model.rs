use serde::{Deserialize, Serialize};

/// Represents a subscription to a specific market or account stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// The subscription ID
    pub id: String,
    /// The type of subscription (MARKET, ACCOUNT, etc.)
    pub subscription_type: SubscriptionType,
    /// The specific item being subscribed to (e.g., market epic)
    pub item: String,
}

/// Types of subscriptions available
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SubscriptionType {
    /// Market data subscription (prices, etc.)
    #[serde(rename = "MARKET")]
    Market,
    /// Account updates (positions, working orders, etc.)
    #[serde(rename = "ACCOUNT")]
    Account,
    /// Trade confirmations
    #[serde(rename = "TRADE")]
    Trade,
    /// Chart data
    #[serde(rename = "CHART")]
    Chart,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    /// Handshake message to establish connection
    #[serde(rename = "HANDSHAKE")]
    Handshake {
        version: String,
        cst: String,
        x_security_token: String,
        origin: String,
    },
    /// Subscribe to a data stream
    #[serde(rename = "SUBSCRIBE")]
    Subscribe {
        subscription: Subscription,
    },
    /// Unsubscribe from a data stream
    #[serde(rename = "UNSUBSCRIBE")]
    Unsubscribe {
        subscription_id: String,
    },
    /// Ping message to keep connection alive
    #[serde(rename = "PING")]
    Ping,
    /// Pong response to ping
    #[serde(rename = "PONG")]
    Pong,
    /// Error message from server
    #[serde(rename = "ERROR")]
    Error {
        code: String,
        message: String,
    },
    /// Data update from a subscription
    #[serde(rename = "UPDATE")]
    Update {
        subscription_id: String,
        data: serde_json::Value,
    },
}

/// Market data update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketUpdate {
    /// Market epic
    pub epic: String,
    /// Current bid price
    pub bid: f64,
    /// Current offer price
    pub offer: f64,
    /// Timestamp of the update
    pub timestamp: String,
}

/// Account update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdate {
    /// Account ID
    pub account_id: String,
    /// Update type (POSITION, ORDER, etc.)
    pub update_type: String,
    /// The data associated with the update
    pub data: serde_json::Value,
}

