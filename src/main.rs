use std::{env, sync::Arc};

use tokio::sync::Mutex;

use dotenv::dotenv;
mod macros;

use crate::{
    binance::{api::BinanceTradingClient, order::BinanceOrderSide},
    constants::{notifications as notif_const, urls},
    models::orderbook::MarketTracker,
    notifications::{alert_gate::AlertGate, telegram::TelegramNotifier},
    ws::{
        binance_client::{self, run_orderbook_stream_binance},
        // binance_client_multiplex::_run_orderbook_stream_binance,
        bybit_client_futures::run_orderbook_stream_bybit_futures,
    },
};

mod binance;
use binance::{create_limit_order, BinanceAuth};

mod constants;
mod logger;
mod models;
pub mod notifications;
mod ws;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let api_key = env::var("API_KEY_BINANCE")
        .or_else(|_| env::var("API_KEY_BINANCE"))
        .expect("API_KEY_BINANCE not set");
    let secret_key = env::var("SECRET_KEY_BINANCE")
        .or_else(|_| env::var("SECRET_KEY_BINANCE"))
        .expect("SECRET_KEY_BINANCE not set");

    let auth = BinanceAuth::new(api_key, secret_key);
    println!(
        "API Key: {}, api secret {}",
        auth.api_key(),
        auth.api_secret()
    );

    // ── Telegram Notifier ────────────────────────────────────────────
    let telegram_tx = TelegramNotifier::spawn();

    // ── Alert Gate (dedup + cooldown) ────────────────────────────────
    let alert_gate = AlertGate::new(
        notif_const::DIFF_THRESHOLD,
        notif_const::RE_ALERT_DELTA,
        notif_const::COOLDOWN_SECS,
    );

    // ── Market Tracker ───────────────────────────────────────────────
    // The comparator threshold is DIFF_THRESHOLD / 100 because the
    // comparator works with a raw ratio multiplied by 100 internally.
    let tracker = Arc::new(Mutex::new(MarketTracker::new(
        notif_const::DIFF_THRESHOLD / 100.0,
        "arbitrage.csv",
        telegram_tx,
        alert_gate,
    )));

    // ── 24-hour state reset scheduler ────────────────────────────────
    {
        let tracker_reset = tracker.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                notif_const::STATE_RESET_SECS,
            ));
            interval.tick().await; // first tick fires immediately — skip it
            loop {
                interval.tick().await;
                tracker_reset.lock().await.alert_gate.reset();
            }
        });
    }

    let mut handles = vec![];

    // --- BYBIT SPOT (DISABLED) ---
    // let symbols_bybit_spot = vec!["WLFIUSDT", "ETHUSDT", "BTCUSDT"];
    // for symbol in symbols_bybit_spot {
    //     let tracker_clone = tracker.clone();
    //     let symbol_owned = symbol.to_string();
    //     handles.push(tokio::spawn(async move {
    //         run_orderbook_stream_bybit(&symbol_owned, tracker_clone, urls::BYBIT_URL_SPOT).await;
    //     }));
    // }

    // --- BYBIT FUTURES ---
    let symbols_bybit_futures = vec![
        "WLFIUSDT",
        "ETHUSDT",
        "BTCUSDT",
        "SOLUSDT",
        "LINKUSDT",
        "XRPUSDT",
        "BNBUSDT",
        "1000PEPEUSDT",
    ];
    for symbol in symbols_bybit_futures {
        let tracker_clone = tracker.clone();
        let symbol_owned = symbol.to_string();
        handles.push(tokio::spawn(async move {
            run_orderbook_stream_bybit_futures(
                &symbol_owned,
                tracker_clone,
                urls::BYBIT_URL_FUTURES_LINEAR,
            )
            .await;
        }));
    }

    // --- BINANCE SPOT (DISABLED) ---
    // let symbols_binance_spot = vec!["wlfiusdt", "ethusdt", "btcusdt"];
    // for symbol in symbols_binance_spot {
    //     let tracker_clone = tracker.clone();
    //     let symbol_owned = symbol.to_string();
    //     handles.push(tokio::spawn(async move {
    //         // Note: binance scanner might need uppercase or lowercase depending on implementation
    //         // Looking at previous code, it seems to handle it or expect lowercase for streams?
    //         // binance_client.rs: line 29: let stream_name = format!("{}@depth", symbol.to_lowercase());
    //         // So casing here doesn't matter too much but let's stick to what we have.
    //         binance_client::run_orderbook_stream_binance(
    //             &symbol_owned,
    //             tracker_clone,
    //             urls::BINANCE_URL_SPOT,
    //         )
    //         .await;
    //     }));
    // }

    // --- BINANCE FUTURES ---
    let symbols_binance_futures = vec![
        "wlfiusdt",
        "ethusdt",
        "btcusdt",
        "solusdt",
        "linkusdt",
        "xrpusdt",
        "bnbusdt",
        "1000pepeusdt",
    ];
    for symbol in symbols_binance_futures {
        let tracker_clone = tracker.clone();
        let symbol_owned = symbol.to_string();
        handles.push(tokio::spawn(async move {
            binance_client::run_orderbook_stream_binance(
                &symbol_owned,
                tracker_clone,
                urls::BINANCE_URL_FUTURES,
            )
            .await;
        }));
    }

    println!("--- Scanning started for: WLFI, ETH, BTC, SOL, LINK, XRP, BNB, 1000PEPE on Binance & Bybit (Spot & Futures) ---");

    // Keep the main thread alive and log heartbeat
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        println!("--- Scanning active: {} ---", chrono::Local::now());
    }
}

async fn test_limit_order_ws(auth: &BinanceAuth) -> Result<(), Box<dyn std::error::Error>> {
    let mut client: BinanceTradingClient =
        BinanceTradingClient::connect(auth.api_key().clone(), auth.api_secret().clone()).await?;

    // --- Order Parameters (Mirroring the Node.js example: LTCUSDT SELL LIMIT @ 90.7) ---
    let order = create_limit_order(
        "LTCUSDT".to_string(),
        BinanceOrderSide::BUY,
        0.23, // quantity
        9.7,  // price
    );

    println!(
        "\n--- Attempting to Place Order ---\nSymbol: {}\nSide: {}\nType: {}\nQuantity: {}\nPrice: {}",
        order.symbol,
        order.side,
        order.order_type,
        order.quantity.unwrap(),
        order.price.unwrap()
    );

    // 2. Place the order
    let place_result = client.future_order_place(&order).await;

    match place_result {
        Ok(result) => {
            println!("\n--- Order Placement SUCCESS ---");
            println!("Order ID: {}", result.order_id);
            println!("Status: {}", result.status);
            println!("Executed Qty: {}", result.executed_qty);

            // 3. Demonstrate checking the order status
            if result.status == "NEW" {
                println!("\n--- Checking Order Status ---");
                let order_id_to_check = result.order_id;

                let status_result = client
                    .future_order_status(result.symbol.clone(), order_id_to_check)
                    .await?;

                println!("Order ID: {}", status_result.order_id);
                println!("Current Status: {}", status_result.status);
                println!("Last Update Time: {}", status_result.update_time);
            }
        }
        Err(e) => {
            eprintln!("\n--- Order Placement FAILED ---");
            eprintln!("Error: {}", e);
        }
    }

    // match client
    //     .future_order_cancel("LTCUSDT".to_string(), 39197978774)
    //     .await
    // {
    //     Ok(result) => {
    //         println!("\n--- Cancellation SUCCESS ---");
    //         println!("Order {} Status: {}", result.order_id, result.status); // Status should be 'CANCELED'
    //     }
    //     Err(e) => {
    //         eprintln!("\n--- Cancellation FAILED ---");
    //         eprintln!("Error: {}", e);
    //     }
    // }

    // In a real application, you would keep the connection open to listen for fills,
    // but for this example, the client handles a single request-response cycle.

    Ok(())
}

// TODO: HERE IS THE PLACEHOLDER FOR THE NEW FUNCTION THAT MAKES FUTURES ORDER CALL AND PLACEC ORDER ON BOTH EXCHANGES SIMULTANEOUSLY
