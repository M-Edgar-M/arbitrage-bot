use futures_util::{SinkExt, StreamExt};
// use serde::{Deserialize, Serialize};
use serde_json::from_str;
// use std::fmt;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{logger::log_binance_orderbook, models::orderbook::BinanceOrderBookMsg};

pub async fn run_orderbook_stream_binance(symbol: &str) {
    // Base URL for Binance Spot WebSocket streams
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
        if let Message::Text(txt) = msg {
            if let Ok(parsed) = from_str::<BinanceOrderBookMsg>(&txt) {
                log_binance_orderbook(&parsed);
            }
        }
    }
}
