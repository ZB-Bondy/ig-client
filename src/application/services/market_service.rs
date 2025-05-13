use std::sync::Arc;
use async_trait::async_trait;
use reqwest::Method;
use tracing::{debug, info};

use crate::{
    application::models::market::{
        HistoricalPricesResponse, MarketDetails, MarketSearchResult,
    },
    config::Config,
    error::AppError,
    session::interface::IgSession,
    transport::http_client::IgHttpClient,
};

/// Interfaz para el servicio de mercado
#[async_trait]
pub trait MarketService: Send + Sync {
    /// Busca mercados por término de búsqueda
    async fn search_markets(&self, session: &IgSession, search_term: &str) -> Result<MarketSearchResult, AppError>;
    
    /// Obtiene detalles de un mercado específico por su EPIC
    async fn get_market_details(&self, session: &IgSession, epic: &str) -> Result<MarketDetails, AppError>;
    
    /// Obtiene precios históricos para un mercado
    async fn get_historical_prices(
        &self,
        session: &IgSession,
        epic: &str,
        resolution: &str,
        from: &str,
        to: &str,
    ) -> Result<HistoricalPricesResponse, AppError>;
}

/// Implementación del servicio de mercado
pub struct MarketServiceImpl<T: IgHttpClient> {
    config: Arc<Config>,
    client: Arc<T>,
}

impl<T: IgHttpClient> MarketServiceImpl<T> {
    /// Crea una nueva instancia del servicio de mercado
    pub fn new(config: Arc<Config>, client: Arc<T>) -> Self {
        Self { config, client }
    }
    
    pub fn get_config(&self) -> &Config {
        &self.config
    }
    
    pub fn set_config(&mut self, config: Arc<Config>) {
        self.config = config;
    }
}

#[async_trait]
impl<T: IgHttpClient + 'static> MarketService for MarketServiceImpl<T> {
    async fn search_markets(&self, session: &IgSession, search_term: &str) -> Result<MarketSearchResult, AppError> {
        let path = format!("markets?searchTerm={}", search_term);
        info!("Buscando mercados con término: {}", search_term);
        
        let result = self.client
            .request::<(), MarketSearchResult>(
                Method::GET,
                &path,
                session,
                None,
                "1",
            )
            .await?;
        
        debug!("Se encontraron {} mercados", result.markets.len());
        Ok(result)
    }
    
    async fn get_market_details(&self, session: &IgSession, epic: &str) -> Result<MarketDetails, AppError> {
        let path = format!("markets/{}", epic);
        info!("Obteniendo detalles del mercado: {}", epic);
        
        let result = self.client
            .request::<(), MarketDetails>(
                Method::GET,
                &path,
                session,
                None,
                "3",
            )
            .await?;
        
        debug!("Detalles del mercado obtenidos para: {}", epic);
        Ok(result)
    }
    
    async fn get_historical_prices(
        &self,
        session: &IgSession,
        epic: &str,
        resolution: &str,
        from: &str,
        to: &str,
    ) -> Result<HistoricalPricesResponse, AppError> {
        let path = format!(
            "prices/{}/{}?from={}&to={}",
            epic, resolution, from, to
        );
        info!("Obteniendo precios históricos para: {}", epic);
        
        let result = self.client
            .request::<(), HistoricalPricesResponse>(
                Method::GET,
                &path,
                session,
                None,
                "3",
            )
            .await?;
        
        debug!("Precios históricos obtenidos para: {}", epic);
        Ok(result)
    }
}

