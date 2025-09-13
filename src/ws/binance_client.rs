use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, time};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    constants::exchange_names,
    models::orderbook::{
        BinanceDepthUpdate, BinanceFuturesOrderBookMsg, BinanceOrderBookMsg, MarketTracker,
        MarketType,
    },
};

pub async fn run_orderbook_stream_binance(
    symbol: &str,
    tracker: Arc<Mutex<MarketTracker>>,
    url: &str,
) {
    loop {
        println!("🔌 Connecting to {}", url);

        let (ws_stream, _) = connect_async(url).await.expect("❌ Failed to connect");
        println!("✅ WebSocket handshake completed for Binance");

        let (mut write, mut read) = ws_stream.split();

        // Subscribe to depth stream
        let stream_name = format!("{}@depth", symbol.to_lowercase());
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": [stream_name],
            "id": 1,
        })
        .to_string();

        write
            .send(Message::Text(subscribe_msg.into()))
            .await
            .unwrap();
        println!("📡 Subscribed to Binance {} orderbook", symbol);

        while let Some(msg_result) = read.next().await {
            let msg = match msg_result {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("❌ WebSocket error: {:?}", e);
                    break;
                }
            };

            if let Message::Text(ref txt) = msg {
                // Ignore subscription ack
                if txt.contains(r#""result":null"#) {
                    continue;
                }

                // Parse JSON manually to determine Spot vs Futures
                let parsed_json: Value = match serde_json::from_str(&txt) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("❌ Failed to parse JSON: {:?}", e);
                        continue;
                    }
                };

                let depth_update =
                    if parsed_json.get("T").is_some() && parsed_json.get("pu").is_some() {
                        // Futures
                        match serde_json::from_value::<BinanceFuturesOrderBookMsg>(parsed_json) {
                            Ok(mut ob) => {
                                ob.market_type = MarketType::Futures;
                                BinanceDepthUpdate::Futures(ob)
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to parse Futures: {:?}", e);
                                continue;
                            }
                        }
                    } else {
                        // Spot
                        match serde_json::from_value::<BinanceOrderBookMsg>(parsed_json) {
                            Ok(mut ob) => {
                                ob.market_type = MarketType::Spot;
                                BinanceDepthUpdate::Spot(ob)
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to parse Spot: {:?}", e);
                                continue;
                            }
                        }
                    };

                // Extract common bids/asks and update tracker
                let (symbol, bids, asks, market_type) = match depth_update {
                    BinanceDepthUpdate::Spot(ob) => (ob.symbol, ob.bids, ob.asks, ob.market_type),
                    BinanceDepthUpdate::Futures(ob) => {
                        (ob.symbol, ob.bids, ob.asks, ob.market_type)
                    }
                };

                if let (Some(bid), Some(ask)) = (bids.get(0), asks.get(0)) {
                    let bid_price: f64 = bid[0].parse().unwrap_or(0.0);
                    let ask_price: f64 = ask[0].parse().unwrap_or(0.0);

                    let mut tracker = tracker.lock().await;
                    tracker.update(
                        exchange_names::BINANCE,
                        &symbol,
                        bid_price,
                        ask_price,
                        market_type,
                    );
                }
            }

            // Respond to Ping
            if let Message::Ping(ref data) = msg {
                write.send(Message::Pong(data.clone())).await.unwrap();
            }
        }

        println!("Connection lost, reconnecting in 10 seconds...");
        time::sleep(Duration::from_secs(10)).await;
    }
}
