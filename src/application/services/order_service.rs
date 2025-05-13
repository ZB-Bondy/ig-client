use std::sync::Arc;
use async_trait::async_trait;
use reqwest::Method;
use tracing::{debug, info};

use crate::{
    application::models::order::{
        ClosePositionRequest, ClosePositionResponse, CreateOrderRequest, CreateOrderResponse,
        OrderConfirmation, UpdatePositionRequest,
    },
    config::Config,
    error::AppError,
    session::interface::IgSession,
    transport::http_client::IgHttpClient,
};

/// Interfaz para el servicio de órdenes
#[async_trait]
pub trait OrderService: Send + Sync {
    /// Crea una nueva orden
    async fn create_order(
        &self,
        session: &IgSession,
        order: &CreateOrderRequest,
    ) -> Result<CreateOrderResponse, AppError>;
    
    /// Obtiene la confirmación de una orden
    async fn get_order_confirmation(
        &self,
        session: &IgSession,
        deal_reference: &str,
    ) -> Result<OrderConfirmation, AppError>;
    
    /// Actualiza una posición existente
    async fn update_position(
        &self,
        session: &IgSession,
        deal_id: &str,
        update: &UpdatePositionRequest,
    ) -> Result<(), AppError>;
    
    /// Cierra una posición existente
    async fn close_position(
        &self,
        session: &IgSession,
        close_request: &ClosePositionRequest,
    ) -> Result<ClosePositionResponse, AppError>;
}

/// Implementación del servicio de órdenes
pub struct OrderServiceImpl<T: IgHttpClient> {
    config: Arc<Config>,
    client: Arc<T>,
}

impl<T: IgHttpClient> OrderServiceImpl<T> {
    /// Crea una nueva instancia del servicio de órdenes
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
impl<T: IgHttpClient + 'static> OrderService for OrderServiceImpl<T> {
    async fn create_order(
        &self,
        session: &IgSession,
        order: &CreateOrderRequest,
    ) -> Result<CreateOrderResponse, AppError> {
        info!("Creando orden para: {}", order.epic);
        
        let result = self.client
            .request::<CreateOrderRequest, CreateOrderResponse>(
                Method::POST,
                "positions/otc",
                session,
                Some(order),
                "2",
            )
            .await?;
        
        debug!("Orden creada con referencia: {}", result.deal_reference);
        Ok(result)
    }
    
    async fn get_order_confirmation(
        &self,
        session: &IgSession,
        deal_reference: &str,
    ) -> Result<OrderConfirmation, AppError> {
        let path = format!("confirms/{}", deal_reference);
        info!("Obteniendo confirmación para la orden: {}", deal_reference);
        
        let result = self.client
            .request::<(), OrderConfirmation>(
                Method::GET,
                &path,
                session,
                None,
                "1",
            )
            .await?;
        
        debug!("Confirmación obtenida para la orden: {}", deal_reference);
        Ok(result)
    }
    
    async fn update_position(
        &self,
        session: &IgSession,
        deal_id: &str,
        update: &UpdatePositionRequest,
    ) -> Result<(), AppError> {
        let path = format!("positions/otc/{}", deal_id);
        info!("Actualizando posición: {}", deal_id);
        
        self.client
            .request::<UpdatePositionRequest, ()>(
                Method::PUT,
                &path,
                session,
                Some(update),
                "2",
            )
            .await?;
        
        debug!("Posición actualizada: {}", deal_id);
        Ok(())
    }
    
    async fn close_position(
        &self,
        session: &IgSession,
        close_request: &ClosePositionRequest,
    ) -> Result<ClosePositionResponse, AppError> {
        info!("Cerrando posición: {}", close_request.deal_id);
        
        let result = self.client
            .request::<ClosePositionRequest, ClosePositionResponse>(
                Method::POST,
                "positions/otc",
                session,
                Some(close_request),
                "1",
            )
            .await?;
        
        debug!("Posición cerrada con referencia: {}", result.deal_reference);
        Ok(result)
    }
}
