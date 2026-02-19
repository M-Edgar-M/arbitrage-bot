use std::{env, sync::Arc};

use tokio::sync::Mutex;

use dotenv::dotenv;
mod macros;

use crate::{
    binance::{
        api::BinanceTradingClient, binance_exchange::BinanceExchange, order::BinanceOrderSide,
    },
    constants::{pairs, thresholds, urls},
    models::orderbook::{Comparator, MarketSnapshot, MarketTracker, MarketType},
    ws::{
        binance_client::{self, run_orderbook_stream_binance},
        // binance_client_multiplex::_run_orderbook_stream_binance,
        bybit_client_futures::run_orderbook_stream_bybit_futures,
        client::run_orderbook_stream_bybit,
        exchanges::OrderSide,
    },
};

mod binance;
use binance::{
    create_limit_order, BinanceAuth, BinanceOrder, NewOrderRespType, OrderType, TimeInForce,
    WorkingType,
};

mod constants;
mod logger;
mod models;
mod ws;

#[tokio::main]
async fn main() {
    // IMPLEMENTATION OF BINANCE SPOT AND FUTURES ORDER
    let snapshots = vec![
        MarketSnapshot {
            exchange: "binance".into(),
            symbol: "BTCUSDT".into(),
            bid: 26000.0,
            ask: 26010.0,
            mid: 26005.0,
            timestamp: chrono::Utc::now().timestamp_millis(),
            // market_type: MarketType::FUTURES,
        },
        MarketSnapshot {
            exchange: "other".into(),
            symbol: "BTCUSDT".into(),
            bid: 26100.0,
            ask: 26120.0,
            mid: 26110.0,
            timestamp: chrono::Utc::now().timestamp_millis(),
            // market_type: MarketType::Spot,
        },
    ];

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
    let ws_url = std::env::var("WS_TESTNET_URL");

    let ws_testnet: &'static str = "wss://ws-api.testnet.binance.vision/ws-api/v3";
    test_limit_order_ws(&auth).await.unwrap();
    // test_market_order_ws(&auth, ws_testnet).await.unwrap();

    // let mut comparator: Comparator = Comparator::new(0.05);
    // comparator
    //     .execute_buy(&auth, ws_testnet, "BTCUSDT", 0.001)
    //     .await
    //     .unwrap();

    // END OF THE BLOCK
    let tracker = Arc::new(Mutex::new(MarketTracker::new(
        // thresholds::LOW_THRESHOLD_1_PERCENT,
        0.00001,
        "arbitrage.csv",
    )));
    let tracker_clone_bybit = tracker.clone();
    let tracker_clone_binance = tracker.clone();
    let tracker_clone_eth_binance = tracker.clone();

    // BYBIT THREAD PICK THREAD BY SEPARATE FUNCTION
    // tokio::spawn(async move {
    //     run_orderbook_stream_bybit("WLFIUSDT", tracker_clone_bybit, urls::BYBIT_URL_SPOT).await;
    // });

    // tokio::spawn(async move {
    //     run_orderbook_stream_bybit(bybit_eth_usdt, tracker.clone()).await;
    // });
    // BINANCE THREAD PICK THREAD BY URL
    // tokio::spawn(async move {
    //     binance_client::run_orderbook_stream_binance(
    //         pairs::WLFI_USDT_BINANCE,
    //         tracker_clone_binance,
    //         urls::BINANCE_URL_FUTURES,
    //     )
    //     .await;
    // });
    // BYBIT FUTURES
    // tokio::spawn(async move {
    //     run_orderbook_stream_bybit_futures(
    //         pairs::WLFI_USDT_BYBIT,
    //         tracker.clone(),
    //         urls::BYBIT_URL_FUTURES,
    //     )
    //     .await;
    // });

    // let symbols = vec![binance_eth_usdt, binance_btc_usdt];
    // tokio::spawn(
    //     async move { _run_orderbook_stream_binance(symbols, tracker_clone_binance).await },
    // );

    // tokio::spawn(async move {
    //     binance_client::run_orderbook_stream_binance(binance_eth_usdt, tracker_clone_eth_binance)
    //         .await;
    // });

    // Keep the main thread alive
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
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
