use std::{sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use serde_json::from_str;
use tokio::{sync::Mutex, time::interval};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    constants::exchange_names,
    // logger,
    models::orderbook::{MarketTracker, MarketType, OrderBookMsg},
};

pub async fn run_orderbook_stream_bybit(
    symbol: &str,
    tracker: Arc<Mutex<MarketTracker>>,
    url: &str,
) {
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

    // Create a periodic interval for sending pings
    let mut ping_interval = interval(Duration::from_secs(20));

    // We'll use a `select` to handle both incoming messages and our ping timer
    loop {
        tokio::select! {
            // This arm handles incoming messages from the WebSocket
            msg = read.next() => {
                let msg = match msg {
                    Some(Ok(msg)) => msg,
                    _ => {
                        println!("Connection closed or error.");
                        break;
                    }
                };
                match msg {
                    Message::Text(txt) => {
                        if let Ok(parsed) = from_str::<OrderBookMsg>(&txt) {
                            if let (Some(bid), Some(ask)) = (parsed.data.b.get(0), parsed.data.a.get(0)) {
                                let bid_price: f64 = bid[0].parse().unwrap_or(0.0);
                                let ask_price: f64 = ask[0].parse().unwrap_or(0.0);


                                let market_type: MarketType = MarketType::Spot;

                                // update the tracker
                                let mut tracker = tracker.lock().await;
                                tracker.update(exchange_names::BYBIT, &parsed.data.s, bid_price, ask_price, market_type);
                            }
                        }
                    },
                    // Handle ping frames sent by the server
                    Message::Ping(data) => {
                        println!("Ping received from server, sending pong back.");
                        if let Err(e) = write.send(Message::Pong(data)).await {
                            eprintln!("Error sending pong: {:?}", e);
                            break;
                        }
                    },
                    _ => {}
                }
            },
            // This arm handles our periodic client-side pings
            _ = ping_interval.tick() => {
                println!("Sending client-side ping.");
                if let Err(e) = write.send(Message::Ping(vec![].into())).await {
                    eprintln!("Error sending ping: {:?}", e);
                    break;
                }
            }
        }
    }
}
