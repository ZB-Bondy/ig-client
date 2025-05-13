use std::sync::Arc;
use async_trait::async_trait;
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, error, info};

use crate::{
    config::Config,
    error::AppError,
    session::interface::IgSession,
};

/// Interface for the IG HTTP client
#[async_trait]
pub trait IgHttpClient: Send + Sync {
    /// Makes an HTTP request to the IG API
    async fn request<T, R>(
        &self,
        method: Method,
        path: &str,
        session: &IgSession,
        body: Option<&T>,
        version: &str,
    ) -> Result<R, AppError>
    where
        for<'de> R: DeserializeOwned + 'static,
        T: Serialize + Send + Sync + 'static;

    /// Makes an unauthenticated HTTP request (for login)
    async fn request_no_auth<T, R>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
        version: &str,
    ) -> Result<R, AppError>
    where
        for<'de> R: DeserializeOwned + 'static,
        T: Serialize + Send + Sync + 'static;
}

/// Implementación del cliente HTTP para IG
pub struct IgHttpClientImpl {
    config: Arc<Config>,
    client: Client,
}

impl IgHttpClientImpl {
    /// Crea una nueva instancia del cliente HTTP
    pub fn new(config: Arc<Config>) -> Self {
        let client = Client::builder()
            .user_agent("ig-client/0.1.0")
            .timeout(std::time::Duration::from_secs(config.rest_api.timeout))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Construye la URL completa para una petición
    fn build_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.config.rest_api.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    /// Añade los headers comunes a todas las peticiones
    fn add_common_headers(&self, builder: RequestBuilder, version: &str) -> RequestBuilder {
        builder
            .header("X-IG-API-KEY", &self.config.credentials.api_key)
            .header("Content-Type", "application/json; charset=UTF-8")
            .header("Accept", "application/json; charset=UTF-8")
            .header("Version", version)
    }

    /// Añade los headers de autenticación a una petición
    fn add_auth_headers(&self, builder: RequestBuilder, session: &IgSession) -> RequestBuilder {
        builder
            .header("CST", &session.cst)
            .header("X-SECURITY-TOKEN", &session.token)
    }

    /// Procesa la respuesta HTTP
    async fn process_response<R>(&self, response: Response) -> Result<R, AppError>
    where
        R: DeserializeOwned,
    {
        let status = response.status();
        let url = response.url().to_string();

        match status {
            StatusCode::OK | StatusCode::CREATED | StatusCode::ACCEPTED => {
                let json = response.json::<R>().await?;
                debug!("Request to {} successful", url);
                Ok(json)
            }
            StatusCode::UNAUTHORIZED => {
                error!("Unauthorized request to {}", url);
                Err(AppError::Unauthorized)
            }
            StatusCode::NOT_FOUND => {
                error!("Resource not found at {}", url);
                Err(AppError::NotFound)
            }
            StatusCode::TOO_MANY_REQUESTS => {
                error!("Rate limit exceeded for {}", url);
                Err(AppError::RateLimitExceeded)
            }
            _ => {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                error!("Request to {} failed with status {}: {}", url, status, error_text);
                Err(AppError::Unexpected(status))
            }
        }
    }
}

#[async_trait]
impl IgHttpClient for IgHttpClientImpl {
    async fn request<T, R>(
        &self,
        method: Method,
        path: &str,
        session: &IgSession,
        body: Option<&T>,
        version: &str,
    ) -> Result<R, AppError>
    where
        for<'de> R: DeserializeOwned + 'static,
        T: Serialize + Send + Sync + 'static,
    {
        let url = self.build_url(path);
        info!("Making {} request to {}", method, url);

        let mut builder = self.client.request(method, &url);
        builder = self.add_common_headers(builder, version);
        builder = self.add_auth_headers(builder, session);

        if let Some(data) = body {
            builder = builder.json(data);
        }

        let response = builder.send().await?;
        self.process_response::<R>(response).await
    }

    async fn request_no_auth<T, R>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
        version: &str,
    ) -> Result<R, AppError>
    where
        for<'de> R: DeserializeOwned + 'static,
        T: Serialize + Send + Sync + 'static,
    {
        let url = self.build_url(path);
        info!("Making unauthenticated {} request to {}", method, url);

        let mut builder = self.client.request(method, &url);
        builder = self.add_common_headers(builder, version);

        if let Some(data) = body {
            builder = builder.json(data);
        }

        let response = builder.send().await?;
        self.process_response::<R>(response).await
    }
}
