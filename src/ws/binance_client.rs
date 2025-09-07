use futures_util::{SinkExt, StreamExt};
use serde_json::from_str;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    constants::exchange_names,
    models::orderbook::{BinanceOrderBookMsg, MarketTracker},
};

pub async fn run_orderbook_stream_binance(symbol: &str, tracker: Arc<Mutex<MarketTracker>>) {
    let url = "wss://stream.binance.com:9443/ws";
    println!("🔌 Connecting to {}", url);

    let (ws_stream, _) = connect_async(url).await.expect("❌ Failed to connect");
    println!("✅ WebSocket handshake completed for Binance");

    let (mut write, mut read) = ws_stream.split();

    // The stream name for a diff depth order book on Binance is <symbol>@depth
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

    while let Some(msg) = read.next().await {
        let msg = msg.unwrap();
        match msg {
            Message::Text(txt) => {
                if let Ok(parsed) = from_str::<BinanceOrderBookMsg>(&txt) {
                    if let (Some(bid), Some(ask)) = (parsed.bids.get(0), parsed.asks.get(0)) {
                        let bid_price: f64 = bid[0].parse().unwrap_or(0.0);
                        let ask_price: f64 = ask[0].parse().unwrap_or(0.0);

                        //update the tracker
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
            // Add a case to handle Ping messages
            Message::Ping(data) => {
                println!("Got a ping, sending a pong!");
                write.send(Message::Pong(data)).await.unwrap();
            }
            // Handle other message types if necessary
            _ => {}
        }
    }
}
