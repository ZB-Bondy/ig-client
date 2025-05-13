use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{self, Receiver, Sender};
// Importamos tokio_tungstenite pero usamos tokio_tungstenite::connect_async directamente
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

    async fn connect_direct(&self, session: &IgSession) -> Result<(), AppError> {
        info!("Using Streaming Companion approach for Lightstreamer connection");
        
        // Basado en lo que hace el Streaming Companion, conectamos directamente al WebSocket
        // con encabezados específicos
        
        // Definir los endpoints de Lightstreamer para intentar conectar
        // Estos son los mismos endpoints que usa el Streaming Companion
        let lightstreamer_endpoints = vec![
            "wss://apd.marketdatasystems.com/lightstreamer",
            "wss://apd145f.marketdatasystems.com/lightstreamer",
            "wss://push.lightstreamer.com/lightstreamer"
        ];
        
        let mut ws_stream = None;
        // Usamos un prefijo _ para indicar que esta variable podría no ser utilizada
        let mut _successful_endpoint = String::new();
        
        // Intentar conectar a cada endpoint
        for endpoint in lightstreamer_endpoints {
            info!("Trying to connect to Lightstreamer endpoint: {}", endpoint);
            
            // Asegurarnos de que estamos usando el esquema WebSocket correcto
            let ws_endpoint = if endpoint.starts_with("https://") {
                endpoint.replace("https://", "wss://")
            } else if endpoint.starts_with("http://") {
                endpoint.replace("http://", "ws://")
            } else if !endpoint.starts_with("wss://") && !endpoint.starts_with("ws://") {
                format!("wss://{}", endpoint)
            } else {
                endpoint.to_string()
            };
            
            info!("Using WebSocket URL: {}", ws_endpoint);
            
            // Usar un enfoque más directo con la URL
            info!("Intentando conexión directa a: {}", ws_endpoint);
            
            // Crear un cliente WebSocket con configuración mínima
            use tokio_tungstenite::tungstenite::client::IntoClientRequest;
            let mut request = ws_endpoint.into_client_request()
                .map_err(|e| {
                    error!("Error creating WebSocket request: {}", e);
                    AppError::WebSocketError(format!("Failed to create WebSocket request: {}", e))
                })?;
                
            // Añadir solo los encabezados necesarios
            request.headers_mut().insert(
                "Sec-WebSocket-Protocol",
                tokio_tungstenite::tungstenite::http::HeaderValue::from_static("js.lightstreamer.com")
            );
            
            request.headers_mut().insert(
                "Origin",
                tokio_tungstenite::tungstenite::http::HeaderValue::from_static("https://labs.ig.com")
            );
                
            // Añadir encabezados de autenticación
            let headers = request.headers_mut();
            headers.insert("CST", tokio_tungstenite::tungstenite::http::HeaderValue::from_str(&session.cst).unwrap());
            headers.insert("X-SECURITY-TOKEN", tokio_tungstenite::tungstenite::http::HeaderValue::from_str(&session.token).unwrap());
            
            // Conectar al servidor WebSocket
            match tokio_tungstenite::connect_async(request).await {
                Ok((stream, response)) => {
                    info!("Successfully connected to: {}", endpoint);
                    info!("HTTP Response: {} {}", response.status(), response.status().canonical_reason().unwrap_or(""));
                    debug!("Response headers: {:#?}", response.headers());
                    
                    ws_stream = Some(stream);
                    _successful_endpoint = endpoint.to_string();
                    break;
                },
                Err(e) => {
                    warn!("Failed to connect to {}: {}", endpoint, e);
                }
            }
        }
        
        // Si no pudimos conectar a ningún endpoint, devolver un error
        if ws_stream.is_none() {
            error!("Failed to connect to any Lightstreamer endpoint");
            return Err(AppError::WebSocketError("Failed to connect to any Lightstreamer endpoint".to_string()));
        }
        
        let ws_stream = ws_stream.unwrap();
        
        // Crear canal para enviar mensajes
        let (tx, mut rx) = mpsc::channel::<Message>(100);
        *self.tx.lock().unwrap() = Some(tx.clone());
        
        // Establecer bandera de conexión
        *self.connected.lock().unwrap() = true;
        
        // Dividir el stream WebSocket
        let (mut ws_tx, mut ws_rx) = ws_stream.split();
        
        // Generar un ID de cliente aleatorio
        let client_id = format!("IGCLIENT_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        
        // Establecer conjunto de adaptadores basado en el entorno
        // Usar exactamente los mismos valores que usa el Streaming Companion
        let adapter_set = if self.config.rest_api.base_url.contains("demo") {
            "DEMO"
        } else {
            "PROD"
        };
        
        info!("Using adapter set: {}", adapter_set);
        info!("Using client ID: {}", client_id);
        
        // Enviar un mensaje de creación de sesión
        // Formato exacto del mensaje que envía el Streaming Companion
        // Nota: El formato es muy específico, incluyendo los saltos de línea y el orden de los parámetros
        let create_session_msg = format!(
            "\r\n\r\nLS_cid={}\r\nLS_send_sync=false\r\nLS_cause=api\r\nLS_adapter_set={}\r\nLS_user={}\r\nLS_password=CST-{}|XST-{}\r\n",
            client_id,
            adapter_set,
            session.account_id,
            session.cst,
            session.token
        );
        
        debug!("Session creation message: {}", create_session_msg.replace("\r\n", "[CR][LF]"));
        match ws_tx.send(Message::Text(create_session_msg.into())).await {
            Ok(_) => info!("Session creation message sent successfully"),
            Err(e) => {
                error!("Error sending session creation message: {}", e);
                return Err(AppError::WebSocketError(format!("Failed to send session creation message: {}", e)));
            }
        }
        
        info!("Waiting for server response...");
        
        // Esperar la respuesta del servidor
        if let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!("Server response: {}", text);
                    
                    // Verificar si la respuesta contiene un error
                    if text.contains("error") || text.contains("Error") || text.contains("ERROR") || text.contains("Cannot continue") {
                        error!("Server returned an error: {}", text);
                        return Err(AppError::WebSocketError(format!("Server returned an error: {}", text)));
                    }
                    
                    // Verificar si la respuesta es LOOP o contiene CONOK (conexión OK)
                    if text.contains("LOOP") {
                        info!("Server requested LOOP, reconnecting...");
                        return self.connect(session).await;
                    } else if !text.contains("CONOK") {
                        warn!("Server response does not contain CONOK, but continuing anyway");
                    }
                }
                Ok(Message::Close(frame)) => {
                    if let Some(frame) = frame {
                        error!("Server closed the connection: {} - {}", frame.code, frame.reason);
                        return Err(AppError::WebSocketError(format!("Server closed the connection: {} - {}", frame.code, frame.reason)));
                    } else {
                        error!("Server closed the connection without a reason");
                        return Err(AppError::WebSocketError("Server closed the connection without a reason".to_string()));
                    }
                }
                Ok(_) => {
                    debug!("Received non-text message from server");
                }
                Err(e) => {
                    error!("Error receiving server response: {}", e);
                    return Err(AppError::WebSocketError(format!("Failed to receive server response: {}", e)));
                }
            }
        } else {
            error!("No response received from server");
            return Err(AppError::WebSocketError("No response received from server".to_string()));
        }
        
        // Iniciar heartbeat
        self.start_heartbeat().await?;
        
        // Ya no necesitamos clonar self porque usamos connected_clone directamente
        
        // Tarea para manejar mensajes entrantes
        let connected_clone = self.connected.clone();
        tokio::spawn(async move {
            while let Some(msg_result) = ws_rx.next().await {
                match msg_result {
                    Ok(msg) => {
                        match msg {
                            Message::Text(text) => {
                                debug!("Received message: {}", text);
                                
                                // Verificar si es un mensaje de error o de cierre
                                if text.contains("error") || text.contains("Error") || text.contains("ERROR") {
                                    error!("Server error: {}", text);
                                    *connected_clone.lock().unwrap() = false;
                                    break;
                                }
                                
                                // Verificar si es un mensaje de LOOP (reconexión)
                                if text.contains("LOOP") {
                                    warn!("Server requested LOOP, connection will be reestablished");
                                    *connected_clone.lock().unwrap() = false;
                                    break;
                                }
                                
                                // Procesar mensajes de actualización de mercado o cuenta
                                // Esto se implementaría en una función separada
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
            
            // Si llegamos aquí, la conexión se ha cerrado
            *connected_clone.lock().unwrap() = false;
            error!("WebSocket connection closed");
        });
        
        // Tarea para enviar mensajes salientes
        tokio::spawn(async move {
            info!("Starting message sending task...");
            while let Some(msg) = rx.recv().await {
                // Mostrar el mensaje a enviar
                if let Message::Text(ref text) = msg {
                    debug!("Sending message: {}", text);
                }
                
                if let Err(e) = ws_tx.send(msg).await {
                    error!("Error sending WebSocket message: {}", e);
                    break;
                }
            }
        });
        
        // Iniciar heartbeat con el formato exacto que espera Lightstreamer
        let heartbeat_interval = tokio::time::Duration::from_secs(30);
        let heartbeat_tx = tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(heartbeat_interval);
            loop {
                interval.tick().await;
                // Enviar un mensaje de heartbeat en el formato que espera Lightstreamer
                let heartbeat_msg = "\r\n\r\nLS_op=hb\r\n";
                if let Err(e) = heartbeat_tx.send(Message::Text(heartbeat_msg.to_string().into())).await {
                    error!("Failed to send heartbeat: {}", e);
                    break;
                }
                debug!("Heartbeat sent");
            }
        });
        
        info!("WebSocket connection established and ready for use");
        
        Ok(())
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
            // Handle messages based on the actual variants in WebSocketMessage
            WebSocketMessage::Subscribe { .. } => {
                debug!("Processed subscribe message");
                // This is an outgoing message, we don't need to process it
            },
            WebSocketMessage::Unsubscribe { .. } => {
                debug!("Processed unsubscribe message");
                // This is an outgoing message, we don't need to process it
            },
            WebSocketMessage::Handshake { .. } => {
                debug!("Processed handshake message");
            },
            WebSocketMessage::Ping => {
                debug!("Processed ping message");
            },
            WebSocketMessage::Pong => {
                debug!("Processed pong message");
            },
            WebSocketMessage::Error { .. } => {
                debug!("Processed error message");
            },
            WebSocketMessage::Update { .. } => {
                debug!("Processed update message");
            },
        }
        
        Ok(())
    }
    
    /// Send a message to the WebSocket server
    async fn send_message(&self, msg: WebSocketMessage) -> Result<(), AppError> {
        // Get a clone of the sender to avoid holding the lock across an await
        let tx_opt = {
            let tx_guard = self.tx.lock().unwrap();
            tx_guard.clone()
        };
        
        if let Some(tx) = tx_opt {
            let json = serde_json::to_string(&msg).map_err(|e| {
                error!("Error serializing message: {}", e);
                AppError::WebSocketError(format!("Failed to serialize message: {}", e))
            })?;
            
            tx.send(Message::Text(json.into())).await.map_err(|e| {
                error!("Error sending message: {}", e);
                AppError::WebSocketError(format!("Failed to send message: {}", e))
            })?;
            
            Ok(())
        } else {
            Err(AppError::WebSocketError("WebSocket not connected".to_string()))
        }
    }
    
    /// Start the heartbeat task
    async fn start_heartbeat(&self) -> Result<(), AppError> {
        let tx_guard = self.tx.lock().unwrap();
        if let Some(tx) = &*tx_guard {
            let tx_clone = tx.clone();
            
            // Spawn a task to send heartbeat messages
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                
                loop {
                    interval.tick().await;
                    
                    // Send a heartbeat message
                    if let Err(e) = tx_clone.send(Message::Ping(vec![].into())).await {
                        error!("Error sending heartbeat: {}", e);
                        break;
                    }
                    
                    debug!("Heartbeat sent");
                }
            });
            
            Ok(())
        } else {
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
        
        // Primero, obtener los detalles de conexión a Lightstreamer desde la API de IG
        let base_url = &self.config.rest_api.base_url;
        let session_url = format!("{}/session", base_url);
        
        info!("Getting session info from: {}", session_url);
        
        // Hacer la solicitud a la API con la API key que usa el Streaming Companion
        let client = reqwest::Client::new();
        let response = client.get(&session_url)
            .header("X-SECURITY-TOKEN", &session.token)
            .header("CST", &session.cst)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("X-IG-API-KEY", "623762bb9bf6e5b3e0675f825ed31413c6a93672")
            .header("Origin", "https://labs.ig.com")
            .header("Referer", "https://labs.ig.com/")
            .send()
            .await
            .map_err(|e| {
                error!("Error requesting session info: {}", e);
                AppError::WebSocketError(format!("Failed to request session info: {}", e))
            })?;
        
        // Verificar el código de estado
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "No response body".to_string());
            error!("Error response from session API: {} - {}", status, text);
            warn!("Falling back to direct connection approach");
            return self.connect_direct(session).await;
        }
        
        // Parsear la respuesta JSON
        let session_info: serde_json::Value = response.json().await.map_err(|e| {
            error!("Error parsing session info: {}", e);
            AppError::WebSocketError(format!("Failed to parse session info: {}", e))
        })?;
        
        // Extraer el endpoint de Lightstreamer
        let lightstreamer_endpoint = match session_info["lightstreamerEndpoint"].as_str() {
            Some(endpoint) => {
                info!("Found Lightstreamer endpoint in session response: {}", endpoint);
                endpoint.to_string()
            },
            None => {
                warn!("Lightstreamer endpoint not found in session response, falling back to direct connection");
                return self.connect_direct(session).await;
            }
        };
        
        // Extraer el client_id
        let client_id = match session_info["clientId"].as_str() {
            Some(id) => {
                info!("Found client ID in session response: {}", id);
                id.to_string()
            },
            None => {
                // Si no hay client_id, generamos uno aleatorio
                let id = format!("IGCLIENT_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
                info!("Client ID not found in session response, using generated ID: {}", id);
                id
            }
        };
        
        // Construir la URL completa para conectar al WebSocket
        // Convertir el esquema https:// a wss://
        let ws_url = if lightstreamer_endpoint.starts_with("https://") {
            format!("wss://{}/lightstreamer", lightstreamer_endpoint.trim_start_matches("https://"))
        } else {
            format!("wss://{}/lightstreamer", lightstreamer_endpoint.trim_start_matches("http://"))
        };
        info!("Connecting to Lightstreamer WebSocket at: {}", ws_url);
        
        // Usar un enfoque más directo con la URL
        info!("Intentando conexión directa a: {}", ws_url);
        
        // Crear un cliente WebSocket con configuración mínima
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let mut request = ws_url.into_client_request()
            .map_err(|e| {
                error!("Error creating WebSocket request: {}", e);
                AppError::WebSocketError(format!("Failed to create WebSocket request: {}", e))
            })?;
            
        // Añadir solo los encabezados necesarios
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            tokio_tungstenite::tungstenite::http::HeaderValue::from_static("js.lightstreamer.com")
        );
        
        request.headers_mut().insert(
            "Origin",
            tokio_tungstenite::tungstenite::http::HeaderValue::from_static("https://labs.ig.com")
        );
        
        // Conectar al servidor WebSocket
        let ws_stream = match tokio_tungstenite::connect_async(request).await {
            Ok((stream, response)) => {
                info!("Successfully connected to: {}", lightstreamer_endpoint);
                info!("HTTP Response: {} {}", response.status(), response.status().canonical_reason().unwrap_or(""));
                debug!("Response headers: {:#?}", response.headers());
                stream
            },
            Err(e) => {
                error!("Failed to connect to {}: {}", lightstreamer_endpoint, e);
                return Err(AppError::WebSocketError(format!("Failed to connect to Lightstreamer: {}", e)));
            }
        };
        
        // Create channel for sending messages
        let (tx, mut rx) = mpsc::channel::<Message>(100);
        *self.tx.lock().unwrap() = Some(tx.clone());
        
        // Set connected flag
        *self.connected.lock().unwrap() = true;
        
        // Split the WebSocket stream
        let (mut ws_tx, mut ws_rx) = ws_stream.split();
        
        // Establecer conjunto de adaptadores basado en el entorno
        // Usar exactamente los mismos valores que usa el Streaming Companion
        let adapter_set = if self.config.rest_api.base_url.contains("demo") {
            "DEMO"
        } else {
            "PROD"
        };
        
        info!("Using adapter set: {}", adapter_set);
        
        // Send a create session message using the credentials from the API response
        // Formato exacto del mensaje que envía el Streaming Companion
        // Nota: El formato es muy específico, incluyendo los saltos de línea y el orden de los parámetros
        let create_session_msg = format!(
            "\r\n\r\nLS_cid={}\r\nLS_send_sync=false\r\nLS_cause=api\r\nLS_adapter_set={}\r\nLS_user={}\r\nLS_password=CST-{}|XST-{}\r\n",
            client_id,
            adapter_set,
            session.account_id,
            session.cst,
            session.token
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
                    debug!("Sending message: {}", text);
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
        if !*self.connected.lock().unwrap() {
            return Ok(());
        }
        
        info!("Disconnecting from WebSocket server...");
        
        // Send a close message - avoiding the Send future issue by dropping the lock before await
        let tx_opt = {
            let tx_guard = self.tx.lock().unwrap();
            tx_guard.clone()
        };
        
        if let Some(tx) = tx_opt {
            tx.send(Message::Close(None)).await.map_err(|e| {
                error!("Error sending close message: {}", e);
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
