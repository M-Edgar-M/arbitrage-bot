use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::from_str;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    logger,
    models::orderbook::{MarketTracker, OrderBookMsg},
};

pub async fn run_orderbook_stream_bybit(symbol: &str, tracker: Arc<Mutex<MarketTracker>>) {
    let url = "wss://stream.bybit.com/v5/public/spot";
    println!("🔌 Connecting to {}", url);

    let (ws_stream, _) = connect_async(url).await.expect("❌ Failed to connect");
    println!("✅ WebSocket handshake completed");

    let (mut write, mut read) = ws_stream.split();

    let subscribe_msg = serde_json::json!({
        "op": "subscribe",
        "args": [format!("orderbook.1.{}", symbol)]
    })
    .to_string();

    write
        .send(Message::Text(subscribe_msg.into()))
        .await
        .unwrap();
    println!("📡 Subscribed to {} orderbook", symbol);

    while let Some(msg) = read.next().await {
        let msg = msg.unwrap();
        if let Message::Text(txt) = msg {
            if let Ok(parsed) = from_str::<OrderBookMsg>(&txt) {
                if let (Some(bid), Some(ask)) = (parsed.data.b.get(0), parsed.data.a.get(0)) {
                    let bid_price: f64 = bid[0].parse().unwrap_or(0.0);
                    let ask_price: f64 = ask[0].parse().unwrap_or(0.0);

                    // update the tracker
                    let mut tracker = tracker.lock().await;
                    tracker.update("Bybit", &parsed.data.s, bid_price, ask_price);
                }
            }
        }
    }
}

// logger::log_orderbook(&parsed);
