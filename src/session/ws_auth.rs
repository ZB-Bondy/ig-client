/******************************************************************************
    Author: Joaquín Béjar García
    Email: jb@taunais.com 
    Date: 4/9/24
 ******************************************************************************/

use crate::config::Config;
use crate::transport::ws_client::WSClient;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, instrument};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Serialize, Deserialize)]
struct WSAuthRequest {
    operation: String,
    client_token: Option<String>,
    account_token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WSAuthResponse {
    operation: String,
    status: String,
    session_id: Option<String>,
}

pub trait WebSocketClient: Send + Sync {
    fn send(&self, message: String) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

impl WebSocketClient for WSClient {
    fn send(&self, message: String) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move { self.send(message).await })
    }
}

pub struct WSAuthSession {
    client: Arc<dyn WebSocketClient>,
    rx: mpsc::Receiver<String>,
    config: Arc<Config>,
}

impl WSAuthSession {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let (client, rx) = WSClient::new(&config);
        Ok(Self {
            client: client as Arc<dyn WebSocketClient>,
            rx,
            config,
        })
    }

    #[instrument(skip(self))]
    pub async fn authenticate(&mut self) -> Result<String> {
        debug!("Starting WebSocket authentication");

        let auth_request = WSAuthRequest {
            operation: "authenticate".to_string(),
            client_token: self.config.credentials.client_token.clone(),
            account_token: self.config.credentials.account_token.clone(),
        };

        let auth_request_json = serde_json::to_string(&auth_request)
            .context("Failed to serialize WebSocket auth request")?;

        debug!("Sending auth request: {}", auth_request_json);

        self.client.send(auth_request_json).await
            .context("Failed to send WebSocket auth request")?;

        debug!("Waiting for auth response");

        while let Some(message) = self.rx.recv().await {
            debug!("Received message: {}", message);
            match serde_json::from_str::<WSAuthResponse>(&message) {
                Ok(response) if response.operation == "authenticate" => {
                    debug!("Received auth response: {:?}", response);
                    match response.status.as_str() {
                        "success" => {
                            debug!("WebSocket authentication successful");
                            return Ok(response.session_id
                                .context("No session ID in successful auth response")?);
                        }
                        _ => {
                            error!("WebSocket authentication failed: {}", response.status);
                            return Err(anyhow::anyhow!("WebSocket authentication failed: {}", response.status));
                        }
                    }
                }
                Ok(_) => {
                    debug!("Received non-auth message, continuing");
                    continue;
                }
                Err(e) => {
                    error!("Failed to parse WebSocket auth response: {:?}", e);
                    return Err(e.into());
                }
            }
        }

        error!("WebSocket connection closed during authentication");
        Err(anyhow::anyhow!("WebSocket connection closed during authentication"))
    }

    pub fn get_client(&self) -> Arc<dyn WebSocketClient> {
        self.client.clone()
    }
}




//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use tokio::sync::mpsc;
//     use tokio::sync::Mutex as TokioMutex;
//     use futures::future::BoxFuture;
//
//     #[derive(Clone)]
//     struct MockWSClient {
//         tx: Arc<TokioMutex<mpsc::Sender<String>>>,
//         received: Arc<TokioMutex<Vec<String>>>,
//     }
//
//     impl MockWSClient {
//         fn new() -> (Self, mpsc::Receiver<String>) {
//             let (tx, rx) = mpsc::channel(100);
//             (Self {
//                 tx: Arc::new(TokioMutex::new(tx)),
//                 received: Arc::new(TokioMutex::new(Vec::new())),
//             }, rx)
//         }
//     }
//
//     impl WebSocketClient for MockWSClient {
//         fn send(&self, message: String) -> BoxFuture<'static, Result<()>> {
//             let tx = self.tx.clone();
//             let received = self.received.clone();
//             Box::pin(async move {
//                 received.lock().await.push(message.clone());
//                 tx.lock().await.send(message).await
//                     .map_err(|e| anyhow::anyhow!("Send error: {:?}", e))
//             })
//         }
//     }
//
//
//
//     fn create_mock_session() -> (WSAuthSession, Arc<MockWSClient>) {
//         let (mock_client, mock_rx) = MockWSClient::new();
//         let mock_client = Arc::new(mock_client);
//         let config = Arc::new(Config::new());
//
//         let ws_auth = WSAuthSession {
//             client: mock_client.clone() as Arc<dyn WebSocketClient>,
//             rx: mock_rx,
//             config,
//         };
//
//         (ws_auth, mock_client)
//     }
//
//     #[tokio::test]
//     async fn test_ws_auth_success() {
//         let (mut ws_auth, mock_client) = create_mock_session();
//
//         tokio::spawn({
//             let tx = mock_client.tx.clone();
//             async move {
//                 let response = WSAuthResponse {
//                     operation: "authenticate".to_string(),
//                     status: "success".to_string(),
//                     session_id: Some("test_session_id".to_string()),
//                 };
//                 let response_json = serde_json::to_string(&response).unwrap();
//                 println!("Sending mock response: {}", response_json);
//                 if let Err(e) = tx.lock().await.send(response_json).await {
//                     println!("Failed to send mock response: {:?}", e);
//                 }
//             }
//         });
//
//         let result = ws_auth.authenticate().await;
//         match result {
//             Ok(session_id) => {
//                 println!("Authentication successful. Session ID: {}", session_id);
//                 assert_eq!(session_id, "test_session_id");
//             },
//             Err(e) => {
//                 println!("Authentication failed: {:?}", e);
//                 panic!("Authentication should have succeeded");
//             }
//         }
//
//         let sent_messages = mock_client.received.lock().await;
//         assert_eq!(sent_messages.len(), 1);
//         let auth_request: WSAuthRequest = serde_json::from_str(&sent_messages[0]).unwrap();
//         println!("Sent auth request: {:?}", auth_request);
//         assert_eq!(auth_request.operation, "authenticate");
//         assert_eq!(auth_request.client_token, Some("test_client_token".to_string()));
//         assert_eq!(auth_request.account_token, Some("test_account_token".to_string()));
//     }
// }