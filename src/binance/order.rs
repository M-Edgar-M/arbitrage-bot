use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;
use std::{fmt, time};

use crate::ws::exchanges::ExchangeError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BinanceOrderSide {
    BUY,
    SELL,
}

impl fmt::Display for BinanceOrderSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BinanceOrderSide::BUY => write!(f, "BUY"),
            BinanceOrderSide::SELL => write!(f, "SELL"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrderType {
    LIMIT,
    MARKET,
    STOP,
    STOP_MARKET,
    TAKE_PROFIT,
    TAKE_PROFIT_MARKET,
    TRAILING_STOP_MARKET,
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OrderType::LIMIT => write!(f, "LIMIT"),
            OrderType::MARKET => write!(f, "MARKET"),
            OrderType::STOP => write!(f, "STOP"),
            OrderType::STOP_MARKET => write!(f, "STOP_MARKET"),
            OrderType::TAKE_PROFIT => write!(f, "TAKE_PROFIT"),
            OrderType::TAKE_PROFIT_MARKET => write!(f, "TAKE_PROFIT_MARKET"),
            OrderType::TRAILING_STOP_MARKET => write!(f, "TRAILING_STOP_MARKET"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TimeInForce {
    GTC,
    IOC,
    FOK,
    GTX,
    GTD,
}

impl fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TimeInForce::GTC => write!(f, "GTC"),
            TimeInForce::IOC => write!(f, "IOC"),
            TimeInForce::FOK => write!(f, "FOK"),
            TimeInForce::GTX => write!(f, "GTX"),
            TimeInForce::GTD => write!(f, "GTD"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PositionSide {
    BOTH,
    LONG,
    SHORT,
}

impl fmt::Display for PositionSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PositionSide::BOTH => write!(f, "BOTH"),
            PositionSide::LONG => write!(f, "LONG"),
            PositionSide::SHORT => write!(f, "SHORT"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WorkingType {
    MARK_PRICE,
    CONTRACT_PRICE,
}

impl fmt::Display for WorkingType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WorkingType::MARK_PRICE => write!(f, "MARK_PRICE"),
            WorkingType::CONTRACT_PRICE => write!(f, "CONTRACT_PRICE"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NewOrderRespType {
    ACK,
    RESULT,
}

impl fmt::Display for NewOrderRespType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NewOrderRespType::ACK => write!(f, "ACK"),
            NewOrderRespType::RESULT => write!(f, "RESULT"),
        }
    }
}

// --- MAIN ORDER STRUCT ---

/// Represents a Binance Futures order request object.
#[derive(Debug, Serialize, Deserialize, Clone)] // Added Serialize
#[serde(rename_all = "camelCase")]
pub struct BinanceOrder {
    pub symbol: String,
    pub side: BinanceOrderSide,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_side: Option<PositionSide>,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_position: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_type: Option<WorkingType>, // Changed type from String to WorkingType
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_protect: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_order_resp_type: Option<NewOrderRespType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
}

impl BinanceOrder {
    /// Convert BinanceOrder to parameters map for API request.
    /// This method only serializes the order parameters, not the authentication fields.
    pub fn to_params(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();

        params.insert("symbol".to_string(), self.symbol.clone());
        params.insert("side".to_string(), self.side.to_string());
        params.insert("type".to_string(), self.order_type.to_string());

        if let Some(position_side) = &self.position_side {
            params.insert("positionSide".to_string(), position_side.to_string());
        }

        if let Some(time_in_force) = &self.time_in_force {
            params.insert("timeInForce".to_string(), time_in_force.to_string());
        }

        if let Some(quantity) = self.quantity {
            params.insert("quantity".to_string(), quantity.to_string());
        }

        if let Some(reduce_only) = self.reduce_only {
            params.insert("reduceOnly".to_string(), reduce_only.to_string());
        }

        if let Some(price) = self.price {
            params.insert("price".to_string(), price.to_string());
        }

        if let Some(stop_price) = self.stop_price {
            params.insert("stopPrice".to_string(), stop_price.to_string());
        }

        if let Some(close_position) = self.close_position {
            params.insert("closePosition".to_string(), close_position.to_string());
        }

        if let Some(activation_price) = self.activation_price {
            params.insert("activationPrice".to_string(), activation_price.to_string());
        }

        if let Some(callback_rate) = self.callback_rate {
            params.insert("callbackRate".to_string(), callback_rate.to_string());
        }

        if let Some(working_type) = &self.working_type {
            params.insert("workingType".to_string(), working_type.to_string());
        }

        if let Some(price_protect) = self.price_protect {
            params.insert("priceProtect".to_string(), price_protect.to_string());
        }

        if let Some(new_order_resp_type) = &self.new_order_resp_type {
            params.insert(
                "newOrderRespType".to_string(),
                new_order_resp_type.to_string(),
            );
        }

        if let Some(client_order_id) = &self.client_order_id {
            params.insert("newClientOrderId".to_string(), client_order_id.clone());
        }

        params
    }
}

// Helper function to create a GTC Limit Order
pub fn create_limit_order(
    symbol: String,
    side: BinanceOrderSide,
    quantity: f64,
    price: f64,
) -> BinanceOrder {
    BinanceOrder {
        symbol,
        side,
        position_side: None,
        order_type: OrderType::LIMIT,
        time_in_force: Some(TimeInForce::GTC),
        quantity: Some(quantity),
        reduce_only: None,
        price: Some(price),
        stop_price: None,
        close_position: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        price_protect: None,
        new_order_resp_type: Some(NewOrderRespType::RESULT),
        client_order_id: None,
    }
}
