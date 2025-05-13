// src/utils/finance.rs
//
// Financial calculation utilities for the IG client

use crate::application::models::account::Position;
use crate::application::models::order::Direction;

/// Calculate the Profit and Loss (P&L) for a position based on current market prices
///
/// # Arguments
///
/// * `position` - The position to calculate P&L for
///
/// # Returns
///
/// * `Option<f64>` - The calculated P&L if market prices are available, None otherwise
///
pub fn calculate_pnl(position: &Position) -> Option<f64> {
    let (bid, offer) = (position.market.bid, position.market.offer);
    
    // Get the appropriate price based on direction
    let current_price = match position.position.direction {
        Direction::Buy => bid,
        Direction::Sell => offer,
    };
    
    // Calculate price difference
    let price_diff = match position.position.direction {
        Direction::Buy => {
            current_price - position.position.level
        }
        Direction::Sell => {
            position.position.level - current_price
        }
    };
    
    // Return P&L
    Some(price_diff * position.position.size)
}

/// Calculate the percentage return for a position
///
/// # Arguments
///
/// * `position` - The position to calculate percentage return for
///
/// # Returns
///
/// * `Option<f64>` - The calculated percentage return if market prices are available, None otherwise
pub fn calculate_percentage_return(position: &Position) -> Option<f64> {
    let pnl = calculate_pnl(position)?;
    let initial_value = position.position.level * position.position.size;
    
    // Avoid division by zero
    if initial_value == 0.0 {
        return None;
    }
    
    Some((pnl / initial_value) * 100.0)
}
