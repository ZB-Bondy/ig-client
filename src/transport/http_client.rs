use anyhow::{Context, Result};
use reqwest::{header, Client};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, error, instrument};

/// Represents the HTTP client for interacting with the IG API.
pub struct IGHttpClient {
    client: Client,
    base_url: String,
}

impl IGHttpClient {
    /// Creates a new instance of the IGHttpClient.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL for the IG API.
    /// * `api_key` - The API key for authentication.
    ///
    /// # Returns
    ///
    /// A Result containing the IGHttpClient instance or an error.
    pub fn new(base_url: &str, api_key: &str) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert("X-IG-API-KEY", header::HeaderValue::from_str(api_key)?);

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            base_url: base_url.to_string(),
        })
    }

    /// Sends a GET request to the specified endpoint.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The API endpoint to send the request to.
    ///
    /// # Returns
    ///
    /// A Result containing the deserialized response or an error.
    #[instrument(skip(self))]
    pub async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending GET request to {}", url);

        let response = self.client.get(&url).send().await?;

        Self::handle_response(response).await
    }

    /// Sends a POST request to the specified endpoint.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The API endpoint to send the request to.
    /// * `body` - The request body to send.
    ///
    /// # Returns
    ///
    /// A Result containing the deserialized response or an error.
    #[instrument(skip(self, body))]
    pub async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        endpoint: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending POST request to {}", url);

        let response = self.client.post(&url).json(body).send().await?;

        Self::handle_response(response).await
    }

    /// Handles the API response.
    ///
    /// # Arguments
    ///
    /// * `response` - The response from the API.
    ///
    /// # Returns
    ///
    /// A Result containing the deserialized response or an error.
    async fn handle_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
        if response.status().is_success() {
            let body = response
                .json()
                .await
                .context("Failed to deserialize response body")?;
            Ok(body)
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .context("Failed to get error response body")?;
            error!("API request failed. Status: {}, Body: {}", status, body);
            anyhow::bail!("API request failed. Status: {}, Body: {}", status, body)
        }
    }
}


#[cfg(test)]
mod tests_ig_http_client {
    use super::*;
    use mockito::{Server, ServerGuard};
    use pretty_assertions::assert_eq;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestResponse {
        message: String,
    }

    #[tokio::test]
    async fn test_new_client() {
        let client = IGHttpClient::new("https://api.ig.com", "test_api_key");
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_get_request_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "success"}"#)
            .create_async()
            .await;

        let client = IGHttpClient::new(&server.url(), "test_api_key").unwrap();
        let response: TestResponse = client.get("/test").await.unwrap();

        assert_eq!(response.message, "success");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_request_failure() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/test")
            .with_status(404)
            .with_body("Not Found")
            .create_async()
            .await;

        let client = IGHttpClient::new(&server.url(), "test_api_key").unwrap();
        let result: Result<TestResponse> = client.get("/test").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API request failed"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_request_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "created"}"#)
            .create_async()
            .await;

        let client = IGHttpClient::new(&server.url(), "test_api_key").unwrap();
        let body = TestResponse { message: "test".to_string() };
        let response: TestResponse = client.post("/test", &body).await.unwrap();

        assert_eq!(response.message, "created");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_request_failure() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(400)
            .with_body("Bad Request")
            .create_async()
            .await;

        let client = IGHttpClient::new(&server.url(), "test_api_key").unwrap();
        let body = TestResponse { message: "test".to_string() };
        let result: Result<TestResponse> = client.post("/test", &body).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API request failed"));
        mock.assert_async().await;
    }
}