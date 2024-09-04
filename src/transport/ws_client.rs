/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 4/9/24
 ******************************************************************************/

use crate::config::{Config, WebSocketConfig};
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};
use tracing::{debug, error, info, instrument};

pub struct WSClient {
    config: WebSocketConfig,
    tx: mpsc::Sender<String>,
}

impl WSClient {
    pub fn new(config: &Config) -> (Arc<Self>, mpsc::Receiver<String>) {
        let (tx, rx) = mpsc::channel(100); // Buffer de 100 mensajes
        (
            Arc::new(Self {
                config: config.websocket.clone(),
                tx,
            }),
            rx,
        )
    }

    #[instrument(skip(self))]
    pub async fn connect_with_retry(self: Arc<Self>) -> Result<()> {
        loop {
            match self.connect().await {
                Ok(()) => {
                    info!("WebSocket connection closed. Reconnecting...");
                }
                Err(e) => {
                    error!("WebSocket connection error: {:?}. Reconnecting...", e);
                }
            }
            sleep(Duration::from_secs(self.config.reconnect_interval)).await;
        }
    }

    async fn connect(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.config.url)
            .await
            .context("WebSocket handshake failed")?;
        debug!("WebSocket connection established");

        let (mut write, read) = ws_stream.split();

        let (_outgoing_tx, mut outgoing_rx) = mpsc::channel(100);
        let write_future = async move {
            while let Some(message) = outgoing_rx.recv().await {
                write.send(Message::Text(message)).await
                    .context("Failed to send message")?;
            }
            Ok::<_, anyhow::Error>(())
        };

        let read_future = self.handle_incoming(read);

        tokio::select! {
            result = write_future => {
                if let Err(e) = result {
                    error!("Error in write handler: {:?}", e);
                }
            }
            result = read_future => {
                if let Err(e) = result {
                    error!("Error in read handler: {:?}", e);
                }
            }
        }

        Ok(())
    }

    #[instrument(skip(self, read))]
    async fn handle_incoming(
        &self,
        mut read: futures_util::stream::SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    ) -> Result<()> {
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    debug!("Received message: {}", text);
                    self.tx.send(text).await.context("Failed to send message to channel")?;
                }
                Ok(Message::Binary(data)) => {
                    debug!("Received binary data of length: {}", data.len());
                    // Manejar datos binarios si es necesario
                }
                Ok(Message::Ping(_)) => {
                    debug!("Received ping");
                    // El manejo de Ping/Pong es automático en tungstenite
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received pong");
                }
                Ok(Message::Close(frame)) => {
                    info!("Received close frame: {:?}", frame);
                    return Ok(());
                }
                Err(e) => {
                    error!("Error receiving message: {:?}", e);
                    return Err(e.into());
                }
                _ => {
                    debug!("Received non-text message");
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    pub async fn send(&self, message: String) -> Result<()> {
        self.tx.send(message).await.context("Failed to send message to WebSocket")?;
        Ok(())
    }
}


#[cfg(test)]
mod tests_ws_client {
    use super::*;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    async fn setup_mock_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws_stream = accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws_stream.split();

            while let Some(Ok(message)) = read.next().await {
                if let Message::Text(text) = message {
                    write.send(Message::Text(format!("Echo: {}", text))).await.unwrap();
                }
            }
        });

        format!("ws://{}", addr)
    }

    #[tokio::test]
    async fn test_ws_client() {
        let server_url = setup_mock_server().await;

        let mut config = Config::default();
        config.websocket.url = server_url;

        let (client, mut rx) = WSClient::new(&config);

        let client_clone = client.clone();
        tokio::spawn(async move {
            client_clone.connect().await.unwrap();
        });

        client.send("Hello".to_string()).await.unwrap();

        if let Some(response) = rx.recv().await {
            assert_eq!(response, "Hello");
        } else {
            panic!("No response received");
        }
    }
}