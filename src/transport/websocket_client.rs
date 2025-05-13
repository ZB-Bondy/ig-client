use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_tungstenite::{connect_async};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};
use crate::config::Config;
use crate::error::AppError;
use crate::session::interface::IgSession;
use crate::transport::model::{AccountUpdate, MarketUpdate, Subscription, SubscriptionType, WebSocketMessage};
use crate::transport::ws_interface::IgWebSocketClient;

/// Implementation of the WebSocket client
pub struct IgWebSocketClientImpl {
    /// Configuration
    config: Arc<Config>,
    /// Connection state
    connected: Arc<Mutex<bool>>,
    /// Map of active subscriptions
    subscriptions: Arc<Mutex<HashMap<String, Subscription>>>,
    /// Sender for outgoing messages
    tx: Arc<Mutex<Option<Sender<Message>>>>,
    /// Sender for market updates
    market_tx: Sender<MarketUpdate>,
    /// Receiver for market updates
    market_rx: Arc<Mutex<Option<Receiver<MarketUpdate>>>>,
    /// Sender for account updates
    account_tx: Sender<AccountUpdate>,
    /// Receiver for account updates
    account_rx: Arc<Mutex<Option<Receiver<AccountUpdate>>>>,
}

impl IgWebSocketClientImpl {
    /// Create a new WebSocket client
    pub fn new(config: Arc<Config>) -> Self {
        let (market_tx, market_rx) = mpsc::channel(100);
        let (account_tx, account_rx) = mpsc::channel(100);
        
        Self {
            config,
            connected: Arc::new(Mutex::new(false)),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            tx: Arc::new(Mutex::new(None)),
            market_tx,
            market_rx: Arc::new(Mutex::new(Some(market_rx))),
            account_tx,
            account_rx: Arc::new(Mutex::new(Some(account_rx))),
        }
    }
    
    /// Handle incoming WebSocket messages
    async fn handle_message(&self, msg: Message) -> Result<(), AppError> {
        if msg.is_text() {
            let text = msg.to_text().unwrap();
            debug!("Message received: {}", text.replace("\r\n", "[CR][LF]\n"));
            
            // For Lightstreamer messages, we need a different parser
            // For now, we just display the message and try to parse it if it's JSON
            if text.starts_with("{") {
                // Looks like JSON, let's try to parse it
                match serde_json::from_str::<WebSocketMessage>(text) {
                    Ok(ws_msg) => {
                        debug!("Message parsed successfully: {:?}", ws_msg);
                        // Process the message according to its type
                        self.process_message(ws_msg).await?;
                    },
                    Err(e) => {
                        warn!("Could not parse message as JSON: {}", e);
                        // Could be another Lightstreamer format
                        debug!("Message content: {}", text);
                    }
                }
            } else {
                // It's a Lightstreamer message in text format
                debug!("Lightstreamer message: {}", text.replace("\r\n", "[CR][LF]\n"));
                
                // Process the Lightstreamer message
                // For example, look for PROBE, SYNC, etc.
                if text.contains("PROBE") {
                    debug!("Received PROBE message, sending response...");
                    // Here we could send a response if needed
                }
            }
        } else if msg.is_binary() {
            debug!("Received binary message");
        } else if msg.is_ping() {
            debug!("Received PING, sending PONG...");
            // Here we could send a PONG if needed
        } else if msg.is_pong() {
            debug!("Received PONG");
        } else if msg.is_close() {
            info!("Received close message");
        }
            
        
        Ok(())
    }
    
    /// Process a WebSocket message according to its type
    async fn process_message(&self, ws_msg: WebSocketMessage) -> Result<(), AppError> {
        match ws_msg {
            WebSocketMessage::Pong => {
                debug!("Received PONG");
            }
            WebSocketMessage::Error { code, message } => {
                error!("Received error: {} - {}", code, message);
            }
            WebSocketMessage::Update { subscription_id, data } => {
                // Get the subscription type
                let subscription_type = {
                    let subscriptions = self.subscriptions.lock().unwrap();
                    match subscriptions.get(&subscription_id) {
                        Some(sub) => sub.subscription_type.clone(),
                        None => {
                            warn!("Received update for unknown subscription: {}", subscription_id);
                            return Ok(());
                        }
                    }
                };
                
                info!("Received update for subscription {}: type={:?}", 
                      subscription_id, subscription_type);
                
                // Process the update based on subscription type
                match subscription_type {
                    SubscriptionType::Market => {
                        debug!("Market update data: {:?}", data);
                        if let Ok(update) = serde_json::from_value::<MarketUpdate>(data.clone()) {
                            info!("Market update: {:?}", update);
                            if let Err(e) = self.market_tx.send(update).await {
                                warn!("Error sending market update: {}", e);
                            }
                        } else {
                            warn!("Error parsing market update: {:?}", data);
                        }
                    }
                    SubscriptionType::Account | SubscriptionType::Trade => {
                        debug!("Account update data: {:?}", data);
                        if let Ok(update) = serde_json::from_value::<AccountUpdate>(data.clone()) {
                            info!("Account update: {:?}", update);
                            if let Err(e) = self.account_tx.send(update).await {
                                warn!("Error sending account update: {}", e);
                            }
                        } else {
                            warn!("Error parsing account update: {:?}", data);
                        }
                    }
                    _ => {
                        debug!("Received update for unsupported subscription type: {:?}", subscription_type);
                    }
                }
            }
            _ => {
                debug!("Received message: {:?}", ws_msg);
            }
        }
        
        Ok(())
    }
    
    /// Send a message to the WebSocket server
    async fn send_message(&self, msg: WebSocketMessage) -> Result<(), AppError> {
        let json = serde_json::to_string(&msg).map_err(|e| {
            AppError::SerializationError(format!("Failed to serialize WebSocket message: {}", e))
        })?;
        
        // Clone the sender to avoid holding the lock across an await point
        let tx = {
            let tx_guard = self.tx.lock().unwrap();
            match &*tx_guard {
                Some(tx) => tx.clone(),
                None => return Err(AppError::WebSocketError("Not connected".to_string())),
            }
        };
        
        tx.send(Message::Text(json.into())).await.map_err(|e| {
            AppError::WebSocketError(format!("Failed to send WebSocket message: {}", e))
        })
    }
    
    /// Start the heartbeat task
    async fn start_heartbeat(&self) -> Result<(), AppError> {
        let tx = {
            let tx_guard = self.tx.lock().unwrap();
            match &*tx_guard {
                Some(tx) => tx.clone(),
                None => return Err(AppError::WebSocketError("Not connected".to_string())),
            }
        };
        
        let connected = self.connected.clone();
        
        tokio::spawn(async move {
            while *connected.lock().unwrap() {
                // Send a ping every 30 seconds
                tokio::time::sleep(Duration::from_secs(30)).await;
                
                if !*connected.lock().unwrap() {
                    break;
                }
                
                debug!("Sending PING");
                if let Err(e) = tx.send(Message::Text(r#"{"type":"PING"}"#.to_string().into())).await {
                    error!("Failed to send PING: {}", e);
                    break;
                }
            }
        });
        
        Ok(())
    }
}

#[async_trait]
impl IgWebSocketClient for IgWebSocketClientImpl {
    async fn connect(&self, session: &IgSession) -> Result<(), AppError> {
        // Check if already connected
        if *self.connected.lock().unwrap() {
            return Ok(());
        }
        
        info!("Connecting to Lightstreamer server...");
        
        // According to the IG documentation, we need to use a direct approach
        // Determine the correct Lightstreamer endpoint based on environment
        let lightstreamer_endpoint = if self.config.rest_api.base_url.contains("demo") {
            "wss://demo-apd.marketdatasystems.com/lightstreamer"
        } else {
            "wss://push.lightstreamer.com/lightstreamer"
        };
        
        // Generate a random client ID
        let client_id = format!("IGCLIENT_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        
        // Set adapter set based on environment
        let adapter_set = if self.config.rest_api.base_url.contains("demo") {
            "DEMO"
        } else {
            "PROD"
        };
        
        // For password, we'll use the CST and XST tokens
        let password = format!("CST-{}|XST-{}", session.cst, session.token);
        
        info!("Using credentials: clientId={}, adapterSet={}, accountId={}", client_id, adapter_set, session.account_id);
        
        // Now we can connect to the WebSocket directly
        let ws_url = lightstreamer_endpoint;
        info!("Using WebSocket URL: {}", ws_url);
        
        // Connect to the WebSocket server
        info!("Connecting to WebSocket server...");
        let (ws_stream, response) = connect_async(ws_url).await.map_err(|e| {
            error!("Error connecting to WebSocket server: {}", e);
            AppError::WebSocketError(format!("Failed to connect to WebSocket server: {}", e))
        })?;
        
        // Show HTTP response information
        info!("Connected to WebSocket server");
        info!("HTTP Response: {} {}", response.status(), response.status().canonical_reason().unwrap_or(""));
        debug!("Response headers: {:#?}", response.headers());
        
        // Create channel for sending messages
        let (tx, mut rx) = mpsc::channel::<Message>(100);
        *self.tx.lock().unwrap() = Some(tx.clone());
        
        // Set connected flag
        *self.connected.lock().unwrap() = true;
        
        // Split the WebSocket stream
        let (mut ws_tx, mut ws_rx) = ws_stream.split();
        
        // Send a create session message
        let create_session_msg = format!(
            "\r\n\r\nLS_adapter_set={}\r\nLS_cid={}\r\nLS_send_sync=false\r\nLS_cause=api\r\nLS_user={}\r\nLS_password={}\r\n",
            adapter_set,
            client_id,
            session.account_id,  // Use the account ID as the user
            password    // The password is from our CST and XST tokens
        );
        
        info!("Sending session creation message...");
        debug!("Session creation message: {}", create_session_msg.replace("\r\n", "[CR][LF]"));
        
        ws_tx.send(Message::Text(create_session_msg.into())).await.map_err(|e| {
            error!("Error sending session creation message: {}", e);
            AppError::WebSocketError(format!("Failed to send session creation message: {}", e))
        })?;
        
        info!("Session creation message sent successfully");
        
        // Wait for and display the server response
        info!("Waiting for server response...");
        if let Some(msg_result) = ws_rx.next().await {
            match msg_result {
                Ok(msg) => {
                    if let Ok(text) = msg.to_text() {
                        info!("Server response: {}", text.replace("\r\n", "[CR][LF]\n"));
                    } else {
                        info!("Server response (non-text): {:?}", msg);
                    }
                },
                Err(e) => {
                    error!("Error receiving server response: {}", e);
                }
            }
        } else {
            warn!("No response received from server");
        }
        
        // Start heartbeat
        self.start_heartbeat().await?;
        
        // Clone references for the tasks
        let self_clone = self.clone();
        
        // Task for handling incoming messages
        tokio::spawn(async move {
            info!("Starting message reception task...");
            while let Some(msg) = ws_rx.next().await {
                match msg {
                    Ok(msg) => {
                        // Show the received message
                        if let Ok(text) = msg.to_text() {
                            debug!("Message received: {}", text.replace("\r\n", "[CR][LF]\n"));
                        } else {
                            debug!("Message received (non-text): {:?}", msg);
                        }
                        
                        if let Err(e) = self_clone.handle_message(msg).await {
                            error!("Error processing WebSocket message: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
            
            // Connection closed
            *self_clone.connected.lock().unwrap() = false;
            info!("WebSocket connection closed");
        });
        
        // Task for sending outgoing messages
        tokio::spawn(async move {
            info!("Starting message sending task...");
            while let Some(msg) = rx.recv().await {
                // Show the message to be sent
                if let Message::Text(ref text) = msg {
                    debug!("Sending message: {}", text.replace("\r\n", "[CR][LF]\n"));
                } else {
                    debug!("Sending message (non-text): {:?}", msg);
                }
                
                if let Err(e) = ws_tx.send(msg).await {
                    error!("Error sending WebSocket message: {}", e);
                    break;
                }
            }
        });
        
        info!("WebSocket connection established and ready for use");
        
        Ok(())
    }
    
    async fn disconnect(&self) -> Result<(), AppError> {
        // Check if connected
        if !*self.connected.lock().unwrap() {
            return Ok(());
        }
        
        // Set connected flag to false
        *self.connected.lock().unwrap() = false;
        
        // Close the channel
        *self.tx.lock().unwrap() = None;
        
        info!("Disconnected from WebSocket server");
        Ok(())
    }
    
    async fn subscribe_market(&self, epic: &str) -> Result<String, AppError> {
        // Generate a subscription ID
        let subscription_id = format!("MARKET-{}", uuid::Uuid::new_v4());
        
        // Create subscription
        let subscription = Subscription {
            id: subscription_id.clone(),
            subscription_type: SubscriptionType::Market,
            item: epic.to_string(),
        };
        
        // Store subscription
        {
            let mut subscriptions = self.subscriptions.lock().unwrap();
            subscriptions.insert(subscription_id.clone(), subscription.clone());
        }
        
        // Send subscription message
        self.send_message(WebSocketMessage::Subscribe {
            subscription,
        }).await?;
        
        info!("Subscribed to market updates for {}", epic);
        Ok(subscription_id)
    }
    
    async fn subscribe_account(&self) -> Result<String, AppError> {
        // Generate a subscription ID
        let subscription_id = format!("ACCOUNT-{}", uuid::Uuid::new_v4());
        
        // Create subscription
        let subscription = Subscription {
            id: subscription_id.clone(),
            subscription_type: SubscriptionType::Account,
            item: "ACCOUNT".to_string(),
        };
        
        // Store subscription
        {
            let mut subscriptions = self.subscriptions.lock().unwrap();
            subscriptions.insert(subscription_id.clone(), subscription.clone());
        }
        
        // Send subscription message
        self.send_message(WebSocketMessage::Subscribe {
            subscription,
        }).await?;
        
        info!("Subscribed to account updates");
        Ok(subscription_id)
    }
    
    async fn unsubscribe(&self, subscription_id: &str) -> Result<(), AppError> {
        // Check if subscription exists
        {
            let mut subscriptions = self.subscriptions.lock().unwrap();
            if !subscriptions.contains_key(subscription_id) {
                return Err(AppError::WebSocketError(format!("Subscription not found: {}", subscription_id)));
            }
            
            // Remove subscription
            subscriptions.remove(subscription_id);
        }
        
        // Send unsubscribe message
        self.send_message(WebSocketMessage::Unsubscribe {
            subscription_id: subscription_id.to_string(),
        }).await?;
        
        info!("Unsubscribed from {}", subscription_id);
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }
    
    fn market_updates(&self) -> Receiver<MarketUpdate> {
        let mut rx_guard = self.market_rx.lock().unwrap();
        if let Some(rx) = rx_guard.take() {
            return rx;
        }
        
        // Create a new channel if none exists
        let (_, rx) = mpsc::channel::<MarketUpdate>(100);
        // Store the sender for later use
        let _market_tx_clone = self.market_tx.clone();
        rx
    }
    
    fn account_updates(&self) -> Receiver<AccountUpdate> {
        let mut rx_guard = self.account_rx.lock().unwrap();
        if let Some(rx) = rx_guard.take() {
            return rx;
        }
        
        // Create a new channel if none exists
        let (_, rx) = mpsc::channel::<AccountUpdate>(100);
        // Store the sender for later use
        let _account_tx_clone = self.account_tx.clone();
        rx
    }
}

// Implement Clone for IgWebSocketClientImpl
impl Clone for IgWebSocketClientImpl {
    fn clone(&self) -> Self {
        let (market_tx, market_rx) = mpsc::channel(100);
        let (account_tx, account_rx) = mpsc::channel(100);
        
        Self {
            config: self.config.clone(),
            connected: self.connected.clone(),
            subscriptions: self.subscriptions.clone(),
            tx: self.tx.clone(),
            market_tx,
            market_rx: Arc::new(Mutex::new(Some(market_rx))),
            account_tx,
            account_rx: Arc::new(Mutex::new(Some(account_rx))),
        }
    }
}
