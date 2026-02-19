use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::constants::urls;

use super::{auth::BinanceAuth, order::BinanceOrder};

/// Response from the Binance WS API for a placed or queried order.
#[derive(Debug, Serialize, Deserialize)]
pub struct BinanceOrderResponse {
    pub id: String,
    pub status: u16,
    pub result: Option<BinanceOrderResult>,
    pub error: Option<WsError>,
}

/// Details of a successful order operation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceOrderResult {
    pub order_id: u64,
    pub symbol: String,
    pub status: String,
    pub client_order_id: String,
    pub price: String,
    pub avg_price: Option<String>,
    pub orig_qty: String,
    pub executed_qty: String,
    pub cum_qty: Option<String>,
    pub cum_quote: Option<String>,
    pub time_in_force: String,
    pub r#type: String, // 'type' is a reserved keyword in Rust, use r#type
    pub reduce_only: bool,
    pub close_position: bool,
    pub side: String,
    pub position_side: String,
    pub stop_price: String,
    pub working_type: String,
    pub price_protect: bool,
    pub orig_type: String,
    pub price_match: String,
    pub self_trade_prevention_mode: String,
    pub good_till_date: u64,
    pub update_time: u64,
}

/// Error details returned by the Binance WS API.
#[derive(Debug, Serialize, Deserialize)]
pub struct WsError {
    pub code: i32,
    pub msg: String,
}

/// A client for interacting with the Binance Futures WebSocket API.
#[derive(Debug)]
pub struct BinanceTradingClient {
    auth: BinanceAuth,
    ws_stream: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
}

impl BinanceTradingClient {
    /// Creates a new BinanceApiClient instance and connects to the WS API.
    ///
    /// # Arguments
    /// * `api_key` - Your Binance API key.
    /// * `api_secret` - Your Binance API secret.
    pub async fn connect(api_key: String, api_secret: String) -> Result<Self> {
        let auth = BinanceAuth::new(api_key, api_secret);
        println!(
            "Attempting to connect to Binance WS API: {}",
            urls::BINANCE_URL_FUTURES
        );

        let (ws_stream, _) = connect_async(urls::BINANCE_URL_FUTURES)
            .await
            .expect("❌ Failed to connect");

        println!("[WS] Connection opened successfully.");

        Ok(Self { auth, ws_stream })
    }

    /// Sends a signed request to the Binance WS API and waits for the response.
    ///
    /// # Arguments
    /// * `method` - The WS API method (e.g., "order.place").
    /// * `params_map` - The raw parameters map before signing.
    async fn send_signed_request(
        &mut self,
        method: &str,
        params_map: std::collections::BTreeMap<String, String>,
    ) -> Result<BinanceOrderResponse> {
        // 1. Augment and sign parameters
        let signed_params = self.auth.augment_and_sign_params(params_map);

        // 2. Build the final JSON request payload
        let request_id = Uuid::new_v4().to_string();
        let payload = json!({
            "id": request_id,
            "method": method,
            "params": signed_params,
        });

        let payload_str = serde_json::to_string(&payload)?;

        // 3. Send the request
        println!(
            "\n[Request {}] Sending signed request for method: '{}'",
            request_id, method
        );
        self.ws_stream
            .send(Message::Text(payload_str.into()))
            .await?;

        // 4. Wait for and process the response
        loop {
            let msg = self.ws_stream.next().await;
            match msg {
                Some(Ok(Message::Text(text))) => {
                    let response: Value = serde_json::from_str(&text)?;

                    // Check if the response contains the ID we sent
                    if response["id"].as_str() == Some(&request_id) {
                        println!("[WS] Received Response for ID: {}", request_id);
                        let order_response: BinanceOrderResponse =
                            serde_json::from_value(response)?;
                        return Ok(order_response);
                    } else {
                        // Handle unsolicited messages (like streams if subscribed)
                        println!("[WS] Unsolicited Message: {}", text);
                    }
                }
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {
                    return Err(anyhow::anyhow!("WebSocket connection closed unexpectedly."));
                }
                _ => continue, // Ignore other message types (Ping, Pong, Binary)
            }
        }
    }

    /// Places a new order on Binance Futures.
    pub async fn future_order_place(&mut self, order: &BinanceOrder) -> Result<BinanceOrderResult> {
        // Convert the order struct to the request parameters map
        let params = order.to_params();

        // Send the signed request
        let response = self.send_signed_request("order.place", params).await?;

        match response.result {
            Some(result) => {
                println!("✅ Order Placed Successfully (ID: {})", result.order_id);
                Ok(result)
            }
            None => Err(anyhow::anyhow!(
                "❌ Order Placement Error: {:?}",
                response.error
            )),
        }
    }

    /// Cancels a pending order on Binance Futures.
    ///
    /// # Arguments
    /// * `symbol` - The symbol of the order to cancel (e.g., "LTCUSDT").
    /// * `order_id` - The ID of the order to cancel.
    pub async fn future_order_cancel(
        &mut self,
        symbol: String,
        order_id: u64,
    ) -> Result<BinanceOrderResult> {
        let mut params = std::collections::BTreeMap::new();
        params.insert("symbol".to_string(), symbol);
        params.insert("orderId".to_string(), order_id.to_string());

        let response = self.send_signed_request("order.cancel", params).await?;

        match response.result {
            Some(result) => {
                println!("✅ Order Cancelled Successfully (ID: {})", result.order_id);
                Ok(result)
            }
            None => Err(anyhow::anyhow!(
                "❌ Order Cancellation Error: {:?}",
                response.error
            )),
        }
    }

    /// Checks the status of a specific order on Binance Futures.
    ///
    /// # Arguments
    /// * `symbol` - The symbol of the order (e.g., "LTCUSDT").
    /// * `order_id` - The ID of the order to check.
    pub async fn future_order_status(
        &mut self,
        symbol: String,
        order_id: u64,
    ) -> Result<BinanceOrderResult> {
        let mut params = std::collections::BTreeMap::new();
        params.insert("symbol".to_string(), symbol);
        params.insert("orderId".to_string(), order_id.to_string());

        let response = self.send_signed_request("order.status", params).await?;

        match response.result {
            Some(result) => {
                println!("🔍 Order Status Checked (ID: {})", result.order_id);
                Ok(result)
            }
            None => Err(anyhow::anyhow!(
                "❌ Order Status Check Error: {:?}",
                response.error
            )),
        }
    }
}
