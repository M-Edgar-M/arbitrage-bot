use crate::constants::pairs;

mod constants;
mod logger;
mod models;
mod ws;

#[tokio::main]
async fn main() {
    let bybit_btc_usdt = pairs::BTC_USDT_BYBIT;
    ws::client::run_orderbook_stream_bybit(bybit_btc_usdt).await
    // ws::binance_client::run_orderbook_stream_binance("btcusdt").await

    // Run the Bybit client for BTCUSDT
    // tokio::spawn(async move {
    //     bybit_client::run_orderbook_stream("BTCUSDT").await;
    // });

    // Run the Binance client for BTCUSDT
    // tokio::spawn(async move {
    //     binance_client::run_orderbook_stream("btcusdt").await;
    // });

    // Keep the main thread alive
    // tokio::signal::ctrl_c().await.unwrap();
    // println!("Ctrl+C received, shutting down...");
}
