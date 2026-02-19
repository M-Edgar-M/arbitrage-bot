use crate::binance::api::BinanceTradingClient;
use crate::binance::order::BinanceOrderSide;
use crate::binance::{create_limit_order, BinanceOrder};
use crate::ws::exchanges::{Exchange, ExchangeError, ExchangeId, OrderSide, PriceData};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

fn map_order_side(side: OrderSide) -> BinanceOrderSide {
    match side {
        OrderSide::Buy => BinanceOrderSide::BUY,
        OrderSide::Sell => BinanceOrderSide::SELL,
    }
}

#[derive(Debug)]
pub struct BinanceExchange {
    pub symbol: String,
    pub ws_url: String,
    trading_client: Mutex<BinanceTradingClient>,
}

impl BinanceExchange {
    pub async fn new(
        symbol: &str,
        api_key: String,
        api_secret: String,
    ) -> Result<Self, ExchangeError> {
        let trading_client = BinanceTradingClient::connect(api_key, api_secret)
            .await
            .expect("❌ Failed to connect to Binance");

        Ok(Self {
            symbol: symbol.to_string(),
            ws_url: format!(
                "wss://stream.binance.com:9443/ws/{}@depth",
                symbol.to_lowercase()
            ),
            trading_client: Mutex::new(trading_client),
        })
    }
}

#[async_trait::async_trait]
impl Exchange for BinanceExchange {
    fn id(&self) -> ExchangeId {
        ExchangeId::Binance
    }

    async fn subscribe_prices(&self, tx: Sender<PriceData>) {
        let (ws_tx, mut ws_rx) = tokio::sync::mpsc::channel(32);

        let handler = crate::binance::ws_handler::WsHandler::new(self.ws_url.clone(), ws_tx);
        handler.start().await;

        while let Some(msg_result) = ws_rx.recv().await {
            match msg_result {
                Ok(Message::Text(txt)) => {
                    let parsed: Value = match serde_json::from_str(&txt) {
                        Ok(v) => v,
                        Err(_) => continue, // Ignore non-JSON messages
                    };

                    // Extract top-of-book
                    if let (Some(bids), Some(asks)) = (parsed.get("b"), parsed.get("a")) {
                        if let (Some(bid), Some(ask)) = (bids.get(0), asks.get(0)) {
                            if let (Some(bid_price_str), Some(ask_price_str)) =
                                (bid.get(0), ask.get(0))
                            {
                                let bid =
                                    bid_price_str.as_str().unwrap_or("0").parse().unwrap_or(0.0);
                                let ask =
                                    ask_price_str.as_str().unwrap_or("0").parse().unwrap_or(0.0);

                                if bid == 0.0 || ask == 0.0 {
                                    continue;
                                }

                                let data = PriceData {
                                    exchange: ExchangeId::Binance,
                                    symbol: self.symbol.clone(),
                                    bid,
                                    ask,
                                };

                                if tx.send(data).await.is_err() {
                                    eprintln!("⚠️ Price channel closed. Exiting Binance task.");
                                    handler.shutdown(); // Stop the WS handler
                                    return; // Exit task completely
                                }
                            }
                        }
                    }
                }
                Ok(Message::Ping(_))
                | Ok(Message::Pong(_))
                | Ok(Message::Binary(_))
                | Ok(Message::Frame(_)) => {
                    // Ignored or handled by tungstenite/handler
                }
                Ok(Message::Close(_)) => {
                    println!("⚠️ Binance task received Close message");
                }
                Err(e) => {
                    eprintln!("❌ WebSocket error from handler: {}", e);
                    // The handler tries to reconnect indefinitely, but if it sends an error,
                    // it might be critical or just a notification.
                    // For now we just log.
                }
            }
        }
        println!("❌ Binance Exchange task finished (channel closed)");
    }
    async fn place_order_future(
        &self,
        side: OrderSide,
        price: f64,
        qty: f64,
    ) -> Result<String, ExchangeError> {
        let binance_side: BinanceOrderSide = map_order_side(side);
        println!(
            "📤 Placing {:?} limit order on Binance: price = {}, qty = {}",
            binance_side, price, qty
        );

        let order: BinanceOrder = create_limit_order(self.symbol.clone(), binance_side, qty, price);
        println!("Order payload: {:?}", order);
        let mut client = self.trading_client.lock().await;

        match client.future_order_place(&order).await {
            Ok(result) => {
                println!("✅ Order Placed Successfully (ID: {})", result.order_id);
                Ok(result.order_id.to_string())
            }
            Err(e) => {
                eprintln!("❌ Order placement failed: {:?}", e);
                Err(ExchangeError::OrderFailed(e.to_string()))
            }
        }
    }
}
