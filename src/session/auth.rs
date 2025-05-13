// src/session/ig_auth.rs  (o donde te encaje)

use async_trait::async_trait;
use reqwest::{Client, StatusCode};

use crate::{
    config::Config,                      // <─ tu struct de antes
    error::AuthError,                    // mismo enum/impl que ya usas
    session::interface::{IgAuthenticator, IgSession},
    session::session::SessionResp,
};

/// Mantiene una referencia a la Config global
pub struct IgAuth<'a> {
    cfg:   &'a Config,
    http:  Client,
}

impl<'a> IgAuth<'a> {
    pub fn new(cfg: &'a Config) -> Self {
        Self {
            cfg,
            http: Client::builder()
                .user_agent("ig-rs/0.1")
                .build()
                .expect("reqwest client"),
        }
    }

    /// Devuelve la URL base correcta (demo vs live) según la config
    fn rest_url(&self, path: &str) -> String {
        format!("{}/{}", self.cfg.rest_api.base_url.trim_end_matches('/'), path.trim_start_matches('/'))
    }
}

#[async_trait]
impl<'a> IgAuthenticator for IgAuth<'a> {
    async fn login(&self) -> Result<IgSession, AuthError> {
        let url  = self.rest_url("session");
        let body = serde_json::json!({
            "identifier": self.cfg.credentials.username,
            "password":   self.cfg.credentials.password,
        });

        let resp = self.http
            .post(url)
            .header("X-IG-API-KEY", &self.cfg.credentials.api_key)
            .header("Content-Type", "application/json; charset=UTF-8")
            .header("Accept",       "application/json; charset=UTF-8")
            .header("Version",      "2")
            .json(&body)
            .send()
            .await?;

        match resp.status() {
            StatusCode::OK => {
                let cst   = resp.headers()
                    .get("CST")
                    .and_then(|v| v.to_str().ok())
                    .ok_or(AuthError::Unexpected(StatusCode::OK))?
                    .to_owned();
                let token = resp.headers()
                    .get("X-SECURITY-TOKEN")
                    .and_then(|v| v.to_str().ok())
                    .ok_or(AuthError::Unexpected(StatusCode::OK))?
                    .to_owned();
                let json: SessionResp = resp.json().await?;
                Ok(IgSession { cst, token, account_id: json.account_id })
            }
            StatusCode::UNAUTHORIZED => Err(AuthError::BadCredentials),
            other                     => Err(AuthError::Unexpected(other)),
        }
    }

    async fn refresh(&self, sess: &IgSession) -> Result<IgSession, AuthError> {
        let url = self.rest_url("session/refresh-token");

        let resp = self.http
            .post(url)
            .header("X-IG-API-KEY", &self.cfg.credentials.api_key)
            .header("CST",             &sess.cst)
            .header("X-SECURITY-TOKEN",&sess.token)
            .header("Version",         "3")
            .send()
            .await?;

        if resp.status() == StatusCode::OK {
            let cst   = resp.headers().get("CST").unwrap().to_str().unwrap().into();
            let token = resp.headers().get("X-SECURITY-TOKEN").unwrap().to_str().unwrap().into();
            let json: SessionResp = resp.json().await?;
            Ok(IgSession { cst, token, account_id: json.account_id })
        } else {
            Err(AuthError::Unexpected(resp.status()))
        }
    }
}