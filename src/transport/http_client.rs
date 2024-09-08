use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client, Response};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::time::Duration;
use log::warn;
use reqwest::header::HeaderMap;
use tracing::{debug, error, instrument};

#[derive(Debug)]
pub(crate)  struct SecurityHeaders {
    pub(crate) cst: Option<String>,
    pub(crate) x_security_token: Option<String>,
    pub(crate) ig_account_id: Option<String>,
    pub(crate) authorization: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) x_ig_api_key: Option<String>,
}

impl SecurityHeaders {
    pub(crate) fn new(cst: Option<String>,
                      x_security_token: Option<String>,
                      ig_account_id: Option<String>,
                      authorization: Option<String>,
                        version: Option<String>,
                        x_ig_api_key: Option<String>   ) -> Self {
        Self {
            cst,
            x_security_token,
            ig_account_id,
            authorization,
            version,
            x_ig_api_key,
        }
    }

    pub(crate) fn get_v1(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Version".to_string(), "1".to_string());
        headers.insert("X-IG-API-KEY".to_string(), self.x_ig_api_key.to_string());
        headers.insert("CST".to_string(), self.cst.to_string());
        headers.insert("X-SECURITY-TOKEN".to_string(), self.x_security_token.to_string());
        headers
    }

    pub(crate) fn get_v2(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Version".to_string(), "2".to_string());
        headers.insert("X-IG-API-KEY".to_string(), self.x_ig_api_key.to_string());
        headers.insert("IG-ACCOUNT-ID".to_string(), self.ig_account_id.to_string());
        headers.insert("Authorization".to_string(), self.authorization.to_string());
        headers
    }

    pub(crate) fn get_v3(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Version".to_string(), "3".to_string());
        headers.insert("X-IG-API-KEY".to_string(), self.x_ig_api_key.to_string());
        headers.insert("IG-ACCOUNT-ID".to_string(), self.ig_account_id.to_string());
        headers.insert("Authorization".to_string(), self.authorization.to_string());
        headers
    }
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self {
            cst: None,
            x_security_token: None,
            ig_account_id: None,
            authorization: None,
            version: None,
            x_ig_api_key: None,
        }
    }
}

/// Represents the HTTP client for interacting with the IG API.
#[derive(Debug)]
pub struct IGHttpClient {
    client: Client,
    base_url: String,
    security_headers: SecurityHeaders,
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
    pub async fn get<T: DeserializeOwned + Debug>(
        &self,
        endpoint: &str,
        headers: Option<HashMap<String, String>>,
    ) -> Result<(T, HeaderMap)>  {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending GET request to {} with headers {:?}", url, headers.clone().unwrap());

        let mut request = self.client.get(&url);
        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to send GET request: {:?}", e);
                anyhow::bail!("Failed to send GET request: {:?}", e)
            }
        };

        let status = response.status();
        if status.is_success() {
            debug!("Response GET Status: {}", status);
            Self::handle_response::<T>(response).await
        } else {
            anyhow::bail!("GET response status: {:?} {}", status, response.text().await?)
        }
    }

    /// Sends a POST request to the specified endpoint.
    #[instrument(skip(self, body))]
    pub async fn post<T: DeserializeOwned + Debug, B: Serialize>(
        &self,
        endpoint: &str,
        body: &B,
    ) -> Result<(T, HeaderMap)>  {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending POST request to {}", url);

        let response = self.client.post(&url).json(body).send().await?;

        let cst = Self::extract_header(&response, "CST")?;
        let x_security_token = Self::extract_header(&response, "X-SECURITY-TOKEN")?;

        // let body = Self::handle_response(response).await?;
        // Ok((body.unwrap(), cst, x_security_token))

        let status = response.status();
        if status.is_success() {
            let body = Self::handle_response::<T>(response).await?;
            debug!("Response body: {:?}", body);
            Ok((body, cst, x_security_token))
        } else {
            anyhow::bail!("POST response status: {:?} {}", status, response.text().await?)
        }
    }

    /// Sends a POST request with custom headers to the specified endpoint.
    #[instrument(skip(self, body, headers))]
    pub async fn post_with_headers<T: DeserializeOwned + Debug, B: Serialize + Debug>(
        &self,
        endpoint: &str,
        body: &B,
        headers: Option<HashMap<String, String>>,
    ) -> Result<(T, HeaderMap)>  {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending POST request with custom headers to {}", url);

        let body_json = serde_json::to_string(body)?;
        debug!("Serialized Body: {}", body_json);

        let mut request = self.client.post(&url).json(body);
        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
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

        let status = response.status();
        if status.is_success() {
            let body = Self::handle_response::<T>(response).await?;
            debug!("Response body: {:?}", body);
            Ok((body, cst, x_security_token))
        } else {
            anyhow::bail!("POST_WITH_HEADERS response status: {:?} {}", status, response.text().await?)
        }
    }

    /// Sends a PUT request to the specified endpoint.
    #[instrument(skip(self, body))]
    pub async fn put<T: DeserializeOwned + Debug + Default, B: Serialize>(
        &self,
        endpoint: &str,
        body: &B,
        headers: &Option<HashMap<String, String>>,
    ) -> Result<(T, HeaderMap)> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending PUT request to {}", url);

        let mut request = self.client.put(&url).json(body);
        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        let response = request.send().await.context("Failed to send PUT request")?;

        let status = response.status();
        let response_headers = response.headers().clone();

        if status.is_success() {
            let answer = Self::handle_response::<T>(response)
                .await
                .context("Failed to handle successful response")?;
            Ok((answer, response_headers))
        } else {
            let error_body = response.text().await.context("Failed to read error response body")?;
            anyhow::bail!("PUT request failed. Status: {}, Body: {}", status, error_body)
        }
    }

    /// Sends a DELETE request to the specified endpoint.
    #[instrument(skip(self))]
    pub async fn delete<T: DeserializeOwned + Debug>(&self, endpoint: &str) -> Result<(T, HeaderMap)>  {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("Sending DELETE request to {}", url);

        let response = self.client.delete(&url).send().await?;

        let status = response.status();
        if status.is_success() {
            Self::handle_response::<T>(response).await
        } else {
            anyhow::bail!("DELETE status: {:?}", status)
        }
    }

    /// Handles the HTTP response by reading its body and attempting to deserialize
    /// it into the specified type `T`.
    ///
    /// This function performs the following steps:
    /// 1. Reads the status of the HTTP response.
    /// 2. Asynchronously reads the body of the response as text.
    /// 3. Logs the status and body for debugging purposes.
    /// 4. If the response status indicates success, it attempts to parse the body
    ///    as JSON into type `T`.
    /// 5. In case of successful parsing, it returns `Ok(Some(body))`.
    /// 6. If the status is not successful or parsing fails, it logs an error and
    ///    returns `Ok(None)`.
    ///
    /// # Type Parameters
    /// - `T`: The type into which the response body should be deserialized.
    ///         This type must implement both `DeserializeOwned` and `Debug` traits.
    ///
    /// # Arguments
    /// - `response`: The HTTP response to be handled of type `Response`.
    ///
    /// # Returns
    /// - `Result<Option<T>>`: Returns `Ok(Some(T))` if the response is successful
    ///   and the body is successfully parsed as JSON.
    ///   Returns `Ok(None)` if the response status is not successful or JSON parsing fails.
    ///   In both cases, in case of any error encountered during reading or parsing, an
    ///   appropriate context will be provided.
    ///
    async fn handle_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T> {
        let status = response.status();
        if status.is_success() {
            let body_text = response
                .text()
                .await
                .context("Failed to read response body")?;
            debug!("Response Status: {}", status);
            debug!("Response Body: {}", body_text);
            let body: T = serde_json::from_str(&body_text).context("Failed to parse JSON")?;
            Ok(body)
        } else {
            anyhow::bail!("Handling response. Status: {} ", status)
        }
    }

    /// Extracts the value of a specified header from an HTTP response.
    ///
    /// This function takes a reference to a `Response` and a header name as input, and returns
    /// a `Result` containing an `Option` with the value of the header if it is found in the response.
    /// If the header is not found, it returns `Ok(None)`.
    ///
    /// # Parameters
    /// - `response`: A reference to the `Response` object from which to extract the header.
    /// - `header_name`: The name of the header to extract.
    ///
    /// # Returns
    /// - `Ok(Some(String))`: If the header is found, an `Option` containing the header value as a `String`.
    /// - `Ok(None)`: If the header is not found.
    /// - `Err`: If there is an error in processing the header.
    ///
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

    pub(crate) fn extract_x_security_token(headers: &HeaderMap) -> Option<String> {
        headers.get("X-SECURITY-TOKEN")
            .and_then(|value| value.to_str().ok())
            .map(String::from)
    }

    pub(crate) fn extract_cst(headers: &HeaderMap) -> Option<String> {
        headers.get("CST")
            .and_then(|value| value.to_str().ok())
            .map(String::from)
    }
}

impl fmt::Display for IGHttpClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"base_url\":\"{}\"}}", self.base_url)
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
        let result: serde_json::Value = client.get("/test", None).await.unwrap();

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
        let mut headers = HashMap::new();
        headers.insert("Custom-Header".to_string(), "custom_value".to_string());

        let (result, cst, x_security_token) = client
            .post_with_headers::<serde_json::Value, serde_json::Value>(
                "/test",
                &body,
                Some(headers),
            )
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
        let headers = Some(HashMap::new());
        let result: serde_json::Value = client.put("/test", &body, &headers).await.unwrap();

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
        let result: Result<serde_json::Value> = client.get("/error", None).await;

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
        let response: serde_json::Value = client.get("/test-endpoint", None).await.unwrap();
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

        let result: Result<serde_json::Value> = client.get("/test-endpoint", None).await;

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

#[cfg(test)]
mod tests_handle_response {
    use super::*;
    use mockito::{Server, Mock, ServerGuard};
    use serde::Deserialize;
    use reqwest::Client;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestResponse {
        message: String,
    }

    async fn setup_mock_server(status: usize, body: &str) -> (ServerGuard, Mock) {
        let mut server = Server::new_async().await;
        let mock = server.mock("GET", "/test")
            .with_status(status)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create();

        (server, mock)
    }

    #[tokio::test]
    async fn test_handle_response_success() {
        let (server, mock) = setup_mock_server(200, r#"{"message": "success"}"#).await;

        let client = Client::new();
        let response = client.get(&format!("{}/test", server.url())).send().await.unwrap();

        let result: Option<TestResponse> = IGHttpClient::handle_response(response).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), TestResponse { message: "success".to_string() });

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_handle_response_error() {
        let (server, mock) = setup_mock_server(404, r#"{"error": "not found"}"#).await;

        let client = Client::new();
        let response = client.get(&format!("{}/test", server.url())).send().await.unwrap();

        let result: Option<TestResponse> = IGHttpClient::handle_response(response).await.unwrap();

        assert!(result.is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_handle_response_invalid_json() {
        let (server, mock) = setup_mock_server(200, r#"{"message": "invalid json"#).await;

        let client = Client::new();
        let response = client.get(&format!("{}/test", server.url())).send().await.unwrap();

        let result: Result<Option<TestResponse>> = IGHttpClient::handle_response(response).await;

        assert!(result.is_err());

        mock.assert_async().await;
    }
}