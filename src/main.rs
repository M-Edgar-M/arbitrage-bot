use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    constants::{pairs, thresholds},
    models::orderbook::MarketTracker,
    ws::{binance_client, client::run_orderbook_stream_bybit},
};

mod constants;
mod logger;
mod models;
mod ws;

#[tokio::main]
async fn main() {
    let tracker = Arc::new(Mutex::new(MarketTracker::new(
        thresholds::MID_THRESHOLD_5_PERCENT,
        "arbitrage.csv",
    )));
    let tracker_clone_bybit = tracker.clone();
    let tracker_clone_binance = tracker.clone();
    let bybit_btc_usdt = pairs::BTC_USDT_BYBIT;
    let binance_btc_usdt = pairs::BTC_USDT_BINANCE;
    // BYBIT THREAD
    tokio::spawn(async move {
        run_orderbook_stream_bybit(bybit_btc_usdt, tracker_clone_bybit).await;
    });

    // BINANCE THREAD
    tokio::spawn(async move {
        binance_client::run_orderbook_stream_binance(binance_btc_usdt, tracker_clone_binance).await;
    });

    // Keep the main thread alive
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
