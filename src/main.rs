use std::{env, sync::Arc};

use tokio::sync::Mutex;

use dotenv::dotenv;
mod macros;

use crate::{
    binance::{api::BinanceTradingClient, order::BinanceOrderSide},
    constants::urls,
    models::orderbook::MarketTracker,
    ws::{
        binance_client::{self, run_orderbook_stream_binance},
        // binance_client_multiplex::_run_orderbook_stream_binance,
        bybit_client_futures::run_orderbook_stream_bybit_futures,
        // client::run_orderbook_stream_bybit, // This was commented out in the main function, so it's unused.
        // exchanges::OrderSide, // This was unused.
    },
};

mod binance;
use binance::{
    create_limit_order,
    BinanceAuth, // BinanceAuth is used in main and test_limit_order_ws
                 // BinanceOrder, NewOrderRespType, OrderType, TimeInForce, WorkingType, // These were unused.
};

mod constants;
mod logger;
mod models;
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
    // let ws_url = std::env::var("WS_TESTNET_URL");
    // let ws_testnet: &'static str = "wss://ws-api.testnet.binance.vision/ws-api/v3";

    // DISABLE TEST ORDER
    // test_limit_order_ws(&auth).await.unwrap();

    let tracker = Arc::new(Mutex::new(MarketTracker::new(0.00001, "arbitrage.csv")));

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
    let symbols_bybit_futures = vec!["WLFIUSDT", "ETHUSDT", "BTCUSDT"];
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
    let symbols_binance_futures = vec!["wlfiusdt", "ethusdt", "btcusdt"];
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

    println!("--- Scanning started for: WLFI, ETH, BTC on Binance & Bybit (Spot & Futures) ---");

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
