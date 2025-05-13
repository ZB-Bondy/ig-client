use async_trait::async_trait;
use reqwest::Method;
use std::sync::Arc;
use tracing::{debug, info};

use crate::{
    application::models::account::{
        AccountActivity, AccountInfo, Positions, TransactionHistory, WorkingOrders,
    },
    config::Config,
    error::AppError,
    session::interface::IgSession,
    transport::http_client::IgHttpClient,
};

/// Interfaz para el servicio de cuenta
#[async_trait]
pub trait AccountService: Send + Sync {
    /// Obtiene información de todas las cuentas del usuario
    async fn get_accounts(&self, session: &IgSession) -> Result<AccountInfo, AppError>;

    /// Obtiene las posiciones abiertas
    async fn get_positions(&self, session: &IgSession) -> Result<Positions, AppError>;

    /// Obtiene las órdenes de trabajo
    async fn get_working_orders(&self, session: &IgSession) -> Result<WorkingOrders, AppError>;

    /// Obtiene la actividad de la cuenta
    async fn get_activity(
        &self,
        session: &IgSession,
        from: &str,
        to: &str,
    ) -> Result<AccountActivity, AppError>;

    /// Obtiene el historial de transacciones
    async fn get_transactions(
        &self,
        session: &IgSession,
        from: &str,
        to: &str,
        page_size: u32,
        page_number: u32,
    ) -> Result<TransactionHistory, AppError>;
}

/// Implementación del servicio de cuenta
pub struct AccountServiceImpl<T: IgHttpClient> {
    config: Arc<Config>,
    client: Arc<T>,
}

impl<T: IgHttpClient> AccountServiceImpl<T> {
    /// Crea una nueva instancia del servicio de cuenta
    pub fn new(config: Arc<Config>, client: Arc<T>) -> Self {
        Self { config, client }
    }
    
    pub fn get_config(&self) -> Arc<Config> {
        self.config.clone()
    }
    
    pub fn set_config(&mut self, config: Arc<Config>) {
        self.config = config;
    }
}

#[async_trait]
impl<T: IgHttpClient + 'static> AccountService for AccountServiceImpl<T> {
    async fn get_accounts(&self, session: &IgSession) -> Result<AccountInfo, AppError> {
        info!("Obteniendo información de cuentas");

        let result = self
            .client
            .request::<(), AccountInfo>(Method::GET, "accounts", session, None, "1")
            .await?;

        debug!(
            "Información de cuentas obtenida: {} cuentas",
            result.accounts.len()
        );
        Ok(result)
    }

    async fn get_positions(&self, session: &IgSession) -> Result<Positions, AppError> {
        info!("Obteniendo posiciones abiertas");

        let result = self
            .client
            .request::<(), Positions>(Method::GET, "positions", session, None, "2")
            .await?;

        debug!(
            "Posiciones obtenidas: {} posiciones",
            result.positions.len()
        );
        Ok(result)
    }

    async fn get_working_orders(&self, session: &IgSession) -> Result<WorkingOrders, AppError> {
        info!("Obteniendo órdenes de trabajo");

        let result = self
            .client
            .request::<(), WorkingOrders>(Method::GET, "workingorders", session, None, "2")
            .await?;

        debug!(
            "Órdenes de trabajo obtenidas: {} órdenes",
            result.working_orders.len()
        );
        Ok(result)
    }

    async fn get_activity(
        &self,
        session: &IgSession,
        from: &str,
        to: &str,
    ) -> Result<AccountActivity, AppError> {
        let path = format!("history/activity?from={}&to={}", from, to);
        info!("Obteniendo actividad de la cuenta");

        let result = self
            .client
            .request::<(), AccountActivity>(Method::GET, &path, session, None, "3")
            .await?;

        debug!(
            "Actividad de la cuenta obtenida: {} actividades",
            result.activities.len()
        );
        Ok(result)
    }

    async fn get_transactions(
        &self,
        session: &IgSession,
        from: &str,
        to: &str,
        page_size: u32,
        page_number: u32,
    ) -> Result<TransactionHistory, AppError> {
        let path = format!(
            "history/transactions?from={}&to={}&pageSize={}&pageNumber={}",
            from, to, page_size, page_number
        );
        info!("Obteniendo historial de transacciones");

        let result = self
            .client
            .request::<(), TransactionHistory>(Method::GET, &path, session, None, "2")
            .await?;

        debug!(
            "Historial de transacciones obtenido: {} transacciones",
            result.transactions.len()
        );
        Ok(result)
    }
}
