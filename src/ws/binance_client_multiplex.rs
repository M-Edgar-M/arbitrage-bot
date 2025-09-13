use futures_util::{SinkExt, StreamExt};
use serde_json::from_str;
use std::sync::Arc;
use std::time::Duration;
use tokio::{sync::Mutex, time};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::{
    constants::exchange_names,
    models::orderbook::{BinanceOrderBookMsg, MarketTracker},
};

// FOR MULTIPLE ASSETS SUBSCRIBE FOR BINANCE
pub async fn _run_orderbook_stream_binance(symbols: Vec<&str>, tracker: Arc<Mutex<MarketTracker>>) {
    // THIS FOR MULTIPLEX REQUESTS SINCE BINANCE HAS **5** SUBSCRIBE MESSAGE PER SECOND LIMIT
    let url = "wss://stream.binance.com:9443/ws";

    loop {
        println!("🔌 Connecting to {}", url);

        let (ws_stream, _) = connect_async(url).await.expect("❌ Failed to connect");
        println!("✅ WebSocket handshake completed for Binance");

        let (mut write, mut read) = ws_stream.split();

        // Build subscription params for all symbols
        let params: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@depth", s.to_lowercase()))
            .collect();

        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": params,
            "id": 1,
        })
        .to_string();

        write
            .send(Message::Text(subscribe_msg.into()))
            .await
            .unwrap();
        println!("📡 Subscribed to Binance orderbooks: {:?}", symbols);

        while let Some(msg_result) = read.next().await {
            let msg = match msg_result {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("❌ WebSocket error: {:?}", e);
                    time::sleep(Duration::from_secs(5)).await;
                    break; // reconnect
                }
            };

            match msg {
                Message::Text(txt) => {
                    if let Ok(parsed) = from_str::<BinanceOrderBookMsg>(&txt) {
                        if let (Some(bid), Some(ask)) = (parsed.bids.get(0), parsed.asks.get(0)) {
                            let bid_price: f64 = bid[0].parse().unwrap_or(0.0);
                            let ask_price: f64 = ask[0].parse().unwrap_or(0.0);

                            let mut tracker = tracker.lock().await;
                            tracker.update(
                                exchange_names::BINANCE,
                                &parsed.symbol,
                                bid_price,
                                ask_price,
                            );
                        }
                    }
                }
                Message::Ping(data) => {
                    println!("Got a ping, sending a pong!");
                    write.send(Message::Pong(data)).await.unwrap();
                }
                _ => {}
            }
        }

        println!("Connection lost, reconnecting in 5s...");
        time::sleep(Duration::from_secs(5)).await;
    }
}
