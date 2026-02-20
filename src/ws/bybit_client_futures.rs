use futures_util::{SinkExt, StreamExt};
use serde_json::from_str;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, time};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    constants::exchange_names,
    models::orderbook::{MarketTracker, MarketType, OrderBookMsg},
};

pub async fn run_orderbook_stream_bybit_futures(
    symbol: &str,
    tracker: Arc<Mutex<MarketTracker>>,
    url: &str,
) {
    println!("🔌 Connecting to {}", url);

    let (ws_stream, _) = connect_async(url).await.expect("❌ Failed to connect");
    println!("✅ WebSocket handshake completed for Futures");

    let (mut write, mut read) = ws_stream.split();
    // The subscription message for Bybit V5 linear futures is the same format as spot
    let subscribe_msg = serde_json::json!({
        "op": "subscribe",
        "args": [format!("orderbook.1.{}", symbol)]
    })
    .to_string();

    write
        .send(Message::Text(subscribe_msg.into()))
        .await
        .unwrap();
    println!("📡 Subscribed to {} futures orderbook", symbol);

    let mut ping_interval = time::interval(Duration::from_secs(20));

    loop {
        tokio::select! {
            msg = read.next() => {
                let msg = match msg {
                    Some(Ok(msg)) => msg,
                    _ => {
                        println!("Connection closed or error. Reconnecting...");
                        break;
                    }
                };
                match msg {
                    Message::Text(txt) => {
                        if let Ok(mut parsed) = from_str::<OrderBookMsg>(&txt) {
                            // Manually set the market type after deserialization
                            parsed.data.market_type = MarketType::Futures;
                            if let (Some(bid), Some(ask)) = (parsed.data.b.first(), parsed.data.a.first()) {
                                let bid_price: f64 = bid[0].parse().unwrap_or(0.0);
                                let ask_price: f64 = ask[0].parse().unwrap_or(0.0);

                                // Update the tracker with the market type
                                let mut tracker = tracker.lock().await;
                                tracker.update(exchange_names::BYBIT, &parsed.data.s, bid_price, ask_price, parsed.data.market_type);
                            }
                        }
                    },
                    Message::Ping(data) => {
                        // println!("Ping received from server, sending pong back.");
                        if let Err(e) = write.send(Message::Pong(data)).await {
                            eprintln!("Error sending pong: {:?}", e);
                            break;
                        }
                    },
                    _ => {}
                }
            },
            _ = ping_interval.tick() => {
                // println!("Sending client-side ping.");
                if let Err(e) = write.send(Message::Ping(vec![].into())).await {
                    eprintln!("Error sending ping: {:?}", e);
                    break;
                }
            }
        }
    }
}
