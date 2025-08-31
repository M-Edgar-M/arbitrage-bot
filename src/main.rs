mod logger;
mod models;
mod ws;

#[tokio::main]
async fn main() {
    // let url: &str = "wss://stream.bybit.com/v5/public/spot";

    // println!("Connecting to {}", url);

    // let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    // println!("✅ WebSocket handshake completed");

    // let (mut write, mut read) = ws_stream.split();

    // // Subscribe to BTCUSDT orderbook depth 25
    // let subscribe_msg = json!({
    //     "op": "subscribe",
    //     // "args": ["orderbook.1.BTCUSDT"]
    //      "args": ["orderbook.1.SOLUSDT"]
    // })
    // .to_string();

    // write
    //     .send(Message::Text(subscribe_msg.into()))
    //     .await
    //     .expect("Failed to send subscribe message");

    // println!("📡 Subscribed to BTCUSDT orderbook");

    // while let Some(msg) = read.next().await {
    //     let msg = msg.expect("Failed to read message");

    //     if let Message::Text(txt) = msg {
    //         // Print raw message for now
    //         println!("Received: {}", txt);

    //         // Later: parse JSON and extract best bid/ask
    //     }
    // }
    ws::client::run_orderbook_stream("BTCUSDT").await
}
