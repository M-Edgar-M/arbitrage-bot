use futures_util::{SinkExt, StreamExt};
use serde_json::from_str;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{logger, models::orderbook::OrderBookMsg};

pub async fn run_orderbook_stream_bybit(symbol: &str) {
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
                logger::log_orderbook(&parsed);
            }
        }
    }
}
