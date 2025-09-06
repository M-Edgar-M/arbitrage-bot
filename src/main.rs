use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    constants::pairs, models::orderbook::MarketTracker, ws::client::run_orderbook_stream_bybit,
};

mod constants;
mod logger;
mod models;
mod ws;

#[tokio::main]
async fn main() {
    let tracker = Arc::new(Mutex::new(MarketTracker::new(0.1, "arbitrage.csv")));
    let tracker_clone = tracker.clone();
    let bybit_btc_usdt = pairs::BTC_USDT_BYBIT;
    // ws::client::run_orderbook_stream_bybit(bybit_btc_usdt, tracker_clone).await
    tokio::spawn(async move {
        run_orderbook_stream_bybit(bybit_btc_usdt, tracker_clone).await;
    });
    // ws::binance_client::run_orderbook_stream_binance("btcusdt").await

    // Run the Bybit client for BTCUSDT
    // tokio::spawn(async move {
    //     bybit_client::run_orderbook_stream("BTCUSDT").await;
    // });

    // Run the Binance client for BTCUSDT
    // tokio::spawn(async move {
    //     binance_client::run_orderbook_stream("btcusdt").await;
    // });

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }

    // Keep the main thread alive
    // tokio::signal::ctrl_c().await.unwrap();
    // println!("Ctrl+C received, shutting down...");
}
