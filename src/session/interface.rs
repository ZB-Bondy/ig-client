use crate::error::AuthError;

/// src/application/services/ig_auth.rs
#[derive(Debug, Clone)]
pub struct IgSession {
    pub cst: String,
    pub token: String,
    pub account_id: String,
}

#[async_trait::async_trait]
pub trait IgAuthenticator: Send + Sync {
    async fn login(&self) -> Result<IgSession, AuthError>;
    async fn refresh(&self, session: &IgSession) -> Result<IgSession, AuthError>;
}