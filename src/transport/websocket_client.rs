use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{self, Receiver, Sender};
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
    /// Connect directly to the Lightstreamer server
    async fn connect_direct(&self, session: &IgSession) -> Result<(), AppError> {
        info!("Using direct WebSocket connection approach for Lightstreamer");
        
        // Define the endpoints to try
        let endpoints = vec![
            "wss://apd.marketdatasystems.com/lightstreamer",
            "wss://apd145f.marketdatasystems.com/lightstreamer",
            "wss://push.lightstreamer.com/lightstreamer"
        ];
        
        // Generate a unique client ID
        let client_id = format!("IGCLIENT_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        
        // Set adapter set based on environment
        let adapter_sets = if self.config.rest_api.base_url.contains("demo") {
            vec!["DEMO-igindexdemo", "DEMO-igstreamer", "DEMO-iggroup"]
        } else {
            vec!["PROD-igindexlive", "PROD-ig", "PROD-iggroup"]
        };
        
        // Format the password in the exact format expected by Lightstreamer
        // Remove any whitespace and ensure there are no strange characters
        let password = format!("CST-{}|XST-{}", 
            session.cst.trim().replace(" ", ""), 
            session.token.trim().replace(" ", "")
        );
        
        // Try each endpoint
        for endpoint in &endpoints {
            info!("Trying to connect to Lightstreamer endpoint: {}", endpoint);
            
            // Create a WebSocket client with minimal configuration
            use tokio_tungstenite::tungstenite::client::IntoClientRequest;
            let mut request = match endpoint.into_client_request() {
                Ok(req) => req,
                Err(e) => {
                    error!("Error creating WebSocket request: {}", e);
                    continue; // Try the next endpoint
                }
            };
                    
            // Add only the necessary headers
            request.headers_mut().insert(
                "Sec-WebSocket-Protocol",
                tokio_tungstenite::tungstenite::http::HeaderValue::from_static("js.lightstreamer.com")
            );
            
            request.headers_mut().insert(
                "Origin",
                tokio_tungstenite::tungstenite::http::HeaderValue::from_static("https://labs.ig.com")
            );
            
            // Connect to the WebSocket server
            let ws_stream = match tokio_tungstenite::connect_async(request).await {
                Ok((stream, response)) => {
                    info!("Successfully connected to: {}", endpoint);
                    info!("HTTP Response: {} {}", response.status(), response.status().canonical_reason().unwrap_or(""));
                    debug!("Response headers: {:#?}", response.headers());
                    stream
                },
                Err(e) => {
                    error!("Failed to connect to {}: {}", endpoint, e);
                    continue; // Try the next endpoint
                }
            };
            
            // Split the WebSocket stream
            let (mut ws_tx, mut ws_rx) = ws_stream.split();
            
            // Try with each adapter set
            for adapter_set in &adapter_sets {
                info!("Trying with adapter set: {}", adapter_set);
                info!("Using client ID: {}", client_id);
                
                // Send a session creation message
                // Format based on the official Lightstreamer documentation
                let create_session_msg = format!(
                    "\r\n\r\nLS_op2=create\r\nLS_cid={}\r\nLS_adapter_set={}\r\nLS_user={}\r\nLS_password={}\r\n",
                    client_id,
                    adapter_set,
                    session.account_id.trim(),
                    password
                );
                
                debug!("Session creation message: {}", create_session_msg.replace("\r\n", "[CR][LF]"));
                match ws_tx.send(Message::Text(create_session_msg.into())).await {
                    Ok(_) => info!("Session creation message sent successfully"),
                    Err(e) => {
                        error!("Error sending session creation message: {}", e);
                        continue; // Try the next adapter set
                    }
                }
                
                info!("Waiting for server response...");
                
                // Wait for the server response
                if let Some(msg) = ws_rx.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            info!("Server response: {}", text);
                            
                            // Check if the response contains an error
                            if text.contains("error") || text.contains("Error") || text.contains("ERROR") || text.contains("Cannot continue") {
                                error!("Server returned an error: {}", text);
                                continue; // Try the next adapter set
                            }
                            
                            // Check if the response is LOOP or contains CONOK (connection OK)
                            if text.contains("LOOP") {
                                info!("Server requested LOOP, reconnecting...");
                                return self.connect(session).await;
                            } else if !text.contains("CONOK") {
                                warn!("Server response does not contain CONOK, trying next adapter set");
                                continue; // Try the next adapter set
                            }
                            
                            // If we got here, the connection was successful
                            info!("Successfully connected with adapter set: {}", adapter_set);
                            
                            // Create channels for sending/receiving messages
                            let (tx, rx) = mpsc::channel::<Message>(100);
                            *self.tx.lock().unwrap() = Some(tx.clone());
                            
                            // Set connection flag
                            *self.connected.lock().unwrap() = true;
                            
                            // Start heartbeat
                            self.start_heartbeat().await?;
                            
                            // Start tasks for receiving and sending messages
                            self.start_tasks(ws_tx, ws_rx, tx, rx);
                            
                            return Ok(());
                        },
                        Ok(Message::Close(frame)) => {
                            if let Some(frame) = frame {
                                error!("Server closed the connection: {} - {}", frame.code, frame.reason);
                                continue; // Try the next adapter set
                            } else {
                                error!("Server closed the connection without a reason");
                                continue; // Try the next adapter set
                            }
                        },
                        Ok(_) => {
                            debug!("Received non-text message from server");
                            continue; // Try the next adapter set
                        },
                        Err(e) => {
                            error!("Error receiving server response: {}", e);
                            continue; // Try the next adapter set
                        }
                    }
                } else {
                    error!("No response received from server");
                    continue; // Try the next adapter set
                }
            }
            
            // If we got here, all adapter sets failed for this endpoint
            error!("All adapter sets failed for endpoint: {}", endpoint);
        }
        
        // If we got here, all endpoints failed
        error!("All endpoints failed");
        return Err(AppError::WebSocketError("All endpoints and adapter sets failed".to_string()));
    }
    
    /// Start tasks for receiving and sending messages
    fn start_tasks(
        &self,
        mut ws_tx: futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>,
        mut ws_rx: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
        _tx: Sender<Message>,
        mut rx: Receiver<Message>
    ) {
        // Task for handling incoming messages
        let connected_clone = self.connected.clone();
        tokio::spawn(async move {
            while let Some(msg_result) = ws_rx.next().await {
                match msg_result {
                    Ok(msg) => {
                        match msg {
                            Message::Text(text) => {
                                debug!("Received message: {}", text);
                                
                                // Check if it's an error or close message
                                if text.contains("error") || text.contains("Error") || text.contains("ERROR") {
                                    error!("Server error: {}", text);
                                    *connected_clone.lock().unwrap() = false;
                                    break;
                                }
                                
                                // Check if it's a LOOP message (reconnection)
                                if text.contains("LOOP") {
                                    warn!("Server requested LOOP, connection will be reestablished");
                                    *connected_clone.lock().unwrap() = false;
                                    break;
                                }
                                
                                // Process market or account update messages
                                // This would be implemented in a separate function
                            },
                            Message::Close(frame) => {
                                if let Some(frame) = frame {
                                    error!("Server closed the connection: {} - {}", frame.code, frame.reason);
                                } else {
                                    error!("Server closed the connection without a reason");
                                }
                                *connected_clone.lock().unwrap() = false;
                                break;
                            },
                            _ => {
                                debug!("Received non-text message: {:?}", msg);
                            }
                        }
                    },
                    Err(e) => {
                        error!("Error receiving message: {}", e);
                        *connected_clone.lock().unwrap() = false;
                        break;
                    }
                }
            }
            
            // If we got here, the connection has been closed
            *connected_clone.lock().unwrap() = false;
            error!("WebSocket connection closed");
        });
        
        // Task for sending outgoing messages
        tokio::spawn(async move {
            info!("Starting message sending task...");
            while let Some(msg) = rx.recv().await {
                // Show the message to be sent
                if let Message::Text(ref text) = msg {
                    debug!("Sending message: {}", text);
                }
                
                if let Err(e) = ws_tx.send(msg).await {
                    error!("Error sending WebSocket message: {}", e);
                    break;
                }
            }
        });
    }
    
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
            if text.contains("SUBOK") || text.contains("SUBCMD") || text.contains("CONOK") {
                debug!("Lightstreamer control message: {}", text);
            } else {
                // Try to parse as JSON
                match serde_json::from_str::<serde_json::Value>(text) {
                    Ok(json) => {
                        debug!("Parsed JSON message: {}", json);
                        // Process the JSON message
                    },
                    Err(e) => {
                        warn!("Could not parse message as JSON: {}", e);
                        // Could be another Lightstreamer format
                        debug!("Message content: {}", text);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Process a WebSocket message according to its type
    async fn process_message(&self, ws_msg: WebSocketMessage) -> Result<(), AppError> {
        match ws_msg {
            WebSocketMessage::Subscribe { subscription } => {
                // Format and send a subscription message
                let subscription_msg = match subscription.subscription_type {
                    SubscriptionType::Market => {
                        format!("\r\n\r\nLS_op=add\r\nLS_subId={}\r\nLS_mode=MERGE\r\nLS_group=MARKET:{}\r\nLS_schema=PRICE\r\n", 
                            subscription.id, subscription.item)
                    },
                    SubscriptionType::Account => {
                        format!("\r\n\r\nLS_op=add\r\nLS_subId={}\r\nLS_mode=MERGE\r\nLS_group=ACCOUNT:{}\r\nLS_schema=ACCOUNT\r\n", 
                            subscription.id, subscription.item)
                    },
                    SubscriptionType::Trade => {
                        format!("\r\n\r\nLS_op=add\r\nLS_subId={}\r\nLS_mode=MERGE\r\nLS_group=TRADE:{}\r\nLS_schema=TRADE\r\n", 
                            subscription.id, subscription.item)
                    },
                    SubscriptionType::Chart => {
                        format!("\r\n\r\nLS_op=add\r\nLS_subId={}\r\nLS_mode=MERGE\r\nLS_group=CHART:{}\r\nLS_schema=CHART\r\n", 
                            subscription.id, subscription.item)
                    }
                };
                
                // Send the subscription message
                self.send_raw_message(Message::Text(subscription_msg.into())).await?;
            },
            WebSocketMessage::Unsubscribe { subscription_id } => {
                // Format and send an unsubscribe message
                let unsubscribe_msg = format!("\r\n\r\nLS_op=delete\r\nLS_subId={}\r\n", subscription_id);
                
                // Send the unsubscribe message
                self.send_raw_message(Message::Text(unsubscribe_msg.into())).await?;
            },
            WebSocketMessage::Handshake { .. } => {
                // Handshake messages are handled in the connect method
                debug!("Ignoring handshake message as it's handled in connect");
            },
            WebSocketMessage::Ping => {
                // Send a pong message
                self.send_raw_message(Message::Pong(vec![].into())).await?;
            },
            WebSocketMessage::Pong => {
                // Pong messages are just acknowledgements, no action needed
                debug!("Received pong message");
            },
            WebSocketMessage::Error { code, message } => {
                error!("Received error message: {} - {}", code, message);
            },
            WebSocketMessage::Update { .. } => {
                // Updates are handled in the handle_message method
                debug!("Update message handled separately");
            }
        }
        
        Ok(())
    }
    
    /// Send a raw message to the WebSocket server
    async fn send_raw_message(&self, msg: Message) -> Result<(), AppError> {
        // Get a clone of the sender outside the mutex guard scope
        let tx_option = {
            let guard = self.tx.lock().unwrap();
            guard.as_ref().cloned()
        };
        
        if let Some(tx) = tx_option {
            tx.send(msg).await.map_err(|e| {
                error!("Failed to send message: {}", e);
                AppError::WebSocketError(format!("Failed to send message: {}", e))
            })?;
            
            Ok(())
        } else {
            error!("WebSocket not connected");
            Err(AppError::WebSocketError("WebSocket not connected".to_string()))
        }
    }
    
    /// Send a message to the WebSocket server
    async fn send_message(&self, msg: WebSocketMessage) -> Result<(), AppError> {
        if !*self.connected.lock().unwrap() {
            return Err(AppError::WebSocketError("WebSocket not connected".to_string()));
        }
        
        // Process the message according to its type
        self.process_message(msg).await
    }
    
    /// Start the heartbeat task
    async fn start_heartbeat(&self) -> Result<(), AppError> {
        if let Some(tx) = self.tx.lock().unwrap().as_ref() {
            let tx_clone = tx.clone();
            
            // Start a task to send heartbeat messages
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                
                loop {
                    interval.tick().await;
                    
                    // Send a heartbeat message in the format expected by Lightstreamer
                    let heartbeat_msg = "\r\n\r\nLS_op=hb\r\n";
                    if let Err(e) = tx_clone.send(Message::Text(heartbeat_msg.into())).await {
                        error!("Failed to send heartbeat: {}", e);
                        break;
                    }
                    
                    debug!("Heartbeat sent");
                }
            });
            
            Ok(())
        } else {
            error!("WebSocket not connected");
            Err(AppError::WebSocketError("WebSocket not connected".to_string()))
        }
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
        
        // Use the direct WebSocket connection approach
        info!("Using direct WebSocket connection approach...");
        return self.connect_direct(session).await;
    }
    
    async fn disconnect(&self) -> Result<(), AppError> {
        if !*self.connected.lock().unwrap() {
            return Ok(());
        }
        
        info!("Disconnecting from WebSocket server...");
        
        // Send a close message
        let tx_option = {
            // Scope the mutex guard to ensure it's dropped before the await
            let guard = self.tx.lock().unwrap();
            guard.as_ref().cloned()
        };
        
        if let Some(tx) = tx_option {
            tx.send(Message::Close(None)).await.map_err(|e| {
                error!("Failed to send close message: {}", e);
                AppError::WebSocketError(format!("Failed to send close message: {}", e))
            })?;
        }
        
        // Set connected flag
        *self.connected.lock().unwrap() = false;
        
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
        rx
    }
    
    fn account_updates(&self) -> Receiver<AccountUpdate> {
        let mut rx_guard = self.account_rx.lock().unwrap();
        if let Some(rx) = rx_guard.take() {
            return rx;
        }
        
        // Create a new channel if none exists
        let (_, rx) = mpsc::channel::<AccountUpdate>(100);
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
