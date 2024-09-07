use anyhow::{Context, Result};
use reqwest::{header, Client, Response};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use std::time::Duration;
use tracing::{debug, error, instrument};

/// Represents the HTTP client for interacting with the IG API.
#[derive(Debug)]
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

    #[instrument(skip(self))]
    pub async fn get<T: DeserializeOwned + Debug>(&self, endpoint: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending GET request to {}", url);

        let response = match self.client.get(&url).send().await {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to send GET request: {:?}", e);
                anyhow::bail!("Failed to send GET request: {:?}", e)
            }
        };

        Self::handle_response(response).await
    }

    /// Sends a POST request to the specified endpoint.
    #[instrument(skip(self, body))]
    pub async fn post<T: DeserializeOwned + Debug, B: Serialize>(
        &self,
        endpoint: &str,
        body: &B,
    ) -> Result<(T, Option<String>, Option<String>)> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending POST request to {}", url);

        let response = self.client.post(&url).json(body).send().await?;

        let cst = Self::extract_header(&response, "CST")?;
        let x_security_token = Self::extract_header(&response, "X-SECURITY-TOKEN")?;

        let body = Self::handle_response(response).await?;
        Ok((body, cst, x_security_token))
    }

    /// Sends a POST request with custom headers to the specified endpoint.
    #[instrument(skip(self, body, headers))]
    pub async fn post_with_headers<T: DeserializeOwned + Debug, B: Serialize + Debug>(
        &self,
        endpoint: &str,
        body: &B,
        headers: &[(String, String)],
    ) -> Result<(T, Option<String>, Option<String>)> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending POST request with custom headers to {}", url);

        let body_json = serde_json::to_string(body)?;
        debug!("Serialized Body: {}", body_json);

        let mut request = self.client.post(&url).json(body);
        for (key, value) in headers {
            request = request.header(key, value);
        }
        debug!("Request headers: {:?}", request);

        let response = request.send().await?;

        debug!("Response: {:?}", response);

        let cst: Option<String> = Self::extract_header(&response, "CST").unwrap_or_else(|e| {
            error!("Failed to extract CST header: {:?}", e);
            None
        });
        let x_security_token: Option<String> = Self::extract_header(&response, "X-SECURITY-TOKEN")
            .unwrap_or_else(|e| {
                error!("Failed to extract X-SECURITY-TOKEN header: {:?}", e);
                None
            });

        // debug!("CST: {}, X-SECURITY-TOKEN: {}", cst, x_security_token);

        let body = match Self::handle_response(response).await {
            Ok(body) => body,
            Err(e) => {
                error!("Failed to handle response: {:?}", e);
                anyhow::bail!("Failed to handle response: {:?}", e)
            }
        };

        debug!("Response body: {:?}", body);

        Ok((body, cst, x_security_token))
    }

    /// Sends a PUT request to the specified endpoint.
    #[instrument(skip(self, body))]
    pub async fn put<T: DeserializeOwned + Debug, B: Serialize>(
        &self,
        endpoint: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending PUT request to {}", url);

        let response = self.client.put(&url).json(body).send().await?;

        Self::handle_response(response).await
    }

    /// Sends a DELETE request to the specified endpoint.
    #[instrument(skip(self))]
    pub async fn delete<T: DeserializeOwned + Debug>(&self, endpoint: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending DELETE request to {}", url);

        let response = self.client.delete(&url).send().await?;

        Self::handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T> {
        let status = response.status();
        let body_text = response
            .text()
            .await
            .context("Failed to read response body")?;

        debug!("Response Status: {}", status);
        debug!("Response Body: {}", body_text);

        if status.is_success() {
            let body: T =
                serde_json::from_str(&body_text).context("Failed to deserialize response body")?;
            Ok(body)
        } else {
            error!(
                "API request failed. Status: {}, Body: {}",
                status, body_text
            );
            anyhow::bail!(
                "API request failed. Status: {}, Body: {}",
                status,
                body_text
            );
        }
    }

    fn extract_header(response: &Response, header_name: &str) -> Result<Option<String>> {
        match response
            .headers()
            .get(header_name)
            .and_then(|h| h.to_str().ok())
            .map(String::from)
        {
            Some(header_value) => Ok(Some(header_value)),
            None => {
                debug!("Header {} not found in response", header_name);
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests_ig_http_client {
    use super::*;
    use crate::utils::logger::setup_logger;
    use mockito::Server;
    use serde_json::json;
    use tracing::info;

    fn create_client(server: &Server) -> IGHttpClient {
        IGHttpClient::new(&server.url(), "test_api_key").unwrap()
    }

    #[tokio::test]
    async fn test_get_request() {
        setup_logger();
        let mut server = Server::new_async().await;

        let mock = server
            .mock("GET", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "success"}"#)
            .create();

        let client = create_client(&server);
        let result: serde_json::Value = client.get("/test").await.unwrap();

        assert_eq!(result["message"], "success");
        mock.assert();
    }

    #[tokio::test]
    async fn test_post_request() {
        setup_logger();
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header("CST", "test_cst")
            .with_header("X-SECURITY-TOKEN", "test_token")
            .with_body(r#"{"message": "created"}"#)
            .create();

        let client = create_client(&server);
        let body = json!({"key": "value"});

        let (result, cst, x_security_token) = client
            .post::<serde_json::Value, serde_json::Value>("/test", &body)
            .await
            .unwrap();

        info!("Result: {:?}", result);

        assert_eq!(result["message"], "created");
        assert_eq!(cst.unwrap(), "test_cst");
        assert_eq!(x_security_token.unwrap(), "test_token");

        mock.assert();
    }

    #[tokio::test]
    async fn test_post_with_headers_request() {
        setup_logger();
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/test")
            .match_header("Custom-Header", "custom_value")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header("CST", "test_cst")
            .with_header("X-SECURITY-TOKEN", "test_token")
            .with_body(r#"{"message": "created_with_headers"}"#)
            .create();

        let client = create_client(&server);
        let body = json!({"key": "value"});
        let headers = vec![("Custom-Header".to_string(), "custom_value".to_string())];

        let (result, cst, x_security_token) = client
            .post_with_headers::<serde_json::Value, serde_json::Value>("/test", &body, &headers)
            .await
            .unwrap();

        assert_eq!(result["message"], "created_with_headers");
        assert_eq!(cst.unwrap(), "test_cst");
        assert_eq!(x_security_token.unwrap(), "test_token");
        mock.assert();
    }

    #[tokio::test]
    async fn test_put_request() {
        setup_logger();
        let mut server = Server::new_async().await;

        let mock = server
            .mock("PUT", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "updated"}"#)
            .create();

        let client = create_client(&server);
        let body = json!({"key": "new_value"});
        let result: serde_json::Value = client.put("/test", &body).await.unwrap();

        assert_eq!(result["message"], "updated");
        mock.assert();
    }

    #[tokio::test]
    async fn test_delete_request() {
        setup_logger();
        let mut server = Server::new_async().await;

        let mock = server
            .mock("DELETE", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "deleted"}"#)
            .create();

        let client = create_client(&server);
        let result: serde_json::Value = client.delete("/test").await.unwrap();

        assert_eq!(result["message"], "deleted");
        mock.assert();
    }

    #[tokio::test]
    async fn test_error_response() {
        setup_logger();
        let mut server = Server::new_async().await;

        let mock = server
            .mock("GET", "/error")
            .with_status(400)
            .with_body("Bad Request")
            .create();

        let client = create_client(&server);
        let result: Result<serde_json::Value> = client.get("/error").await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("API request failed"));
        mock.assert();
    }

    #[tokio::test]
    async fn test_missing_headers() {
        setup_logger();
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "success"}"#)
            .create();

        let client = create_client(&server);
        let body = json!({"key": "value"});

        let result = client
            .post::<serde_json::Value, serde_json::Value>("/test", &body)
            .await;
        assert!(result.is_ok());
        mock.assert();
    }
}

#[cfg(test)]
mod tests_get {
    use super::*;
    use crate::utils::logger::setup_logger;
    use mockito::Server;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[tokio::test]
    async fn test_get_success() {
        setup_logger();
        let mut server = Server::new_async().await;

        let _mock = server
            .mock("GET", "/test-endpoint")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"key": "value"}"#)
            .create();

        let client = IGHttpClient::new(&server.url(), "test-api-key").unwrap();
        let response: serde_json::Value = client.get("/test-endpoint").await.unwrap();
        assert_eq!(response, json!({"key": "value"}));
    }

    #[tokio::test]
    async fn test_get_error() {
        let mut server = Server::new_async().await;

        let _mock = server
            .mock("GET", "/test-endpoint")
            .with_status(404)
            .with_body("Not Found")
            .create();

        let client = IGHttpClient::new(&server.url(), "test-api-key").unwrap();

        let result: Result<serde_json::Value> = client.get("/test-endpoint").await;

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod tests_post {
    use super::*;
    use crate::utils::logger::setup_logger;
    use mockito::{Matcher, Server}; // Usamos mockito para las solicitudes POST
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[tokio::test]
    async fn test_post_success() {
        setup_logger();
        let mut server = Server::new_async().await;

        let _mock = server
            .mock("POST", "/test-endpoint")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_header("CST", "test-cst")
            .with_header("X-SECURITY-TOKEN", "test-security-token")
            .with_body(r#"{"key": "value"}"#)
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(json!({"request_key": "request_value"})))
            .create();

        let client = IGHttpClient::new(&server.url(), "test-api-key").unwrap();

        let (response, cst, x_security_token): (serde_json::Value, Option<String>, Option<String>) =
            client
                .post("/test-endpoint", &json!({"request_key": "request_value"}))
                .await
                .unwrap();

        assert_eq!(response, json!({"key": "value"}));
        assert_eq!(cst.unwrap(), "test-cst");
        assert_eq!(x_security_token.unwrap(), "test-security-token");
    }

    #[tokio::test]
    async fn test_post_error() {
        setup_logger();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/test-endpoint")
            .with_status(400)
            .with_body("Bad Request")
            .create();

        let client = IGHttpClient::new(&server.url(), "test-api-key").unwrap();

        let result: Result<(serde_json::Value, Option<String>, Option<String>)> = client
            .post("/test-endpoint", &json!({"request_key": "request_value"}))
            .await;

        assert!(result.is_err());
    }
}
