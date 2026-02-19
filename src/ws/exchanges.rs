use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::{self, Sender};
use tokio::time::{self, Duration};

use crate::models::orderbook::{MarketTracker, MarketType, OrderBookMsg};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExchangeId {
    Binance,
    Bybit,
}

// Implement Display for clean printing
impl std::fmt::Display for ExchangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct PriceData {
    pub exchange: ExchangeId,
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
}

#[derive(Debug, Clone)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug)]
pub enum ExchangeError {
    ConnectionFailed(String),
    OrderFailed(String),
    WebSocketError(String),
}

#[async_trait]
pub trait Exchange: Send + Sync {
    fn id(&self) -> ExchangeId;

    async fn subscribe_prices(&self, tx: Sender<PriceData>);

    async fn place_order_future(
        &self,
        side: OrderSide,
        price: f64,
        qty: f64,
    ) -> Result<String, ExchangeError>;
}

pub struct ArbitrageEngine {
    exchanges: HashMap<ExchangeId, Arc<dyn Exchange>>,
    market_state: HashMap<ExchangeId, PriceData>,
    price_rx: mpsc::Receiver<PriceData>,
    threshold: f64, // e.g., 0.001 for 0.1%
    quantity: f64,
    is_executing: bool, // Simple mutex to prevent re-entrancy
}

impl ArbitrageEngine {
    pub fn new(exchange_list: Vec<Arc<dyn Exchange>>, threshold: f64, quantity: f64) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let mut exchanges = HashMap::new();

        for exchange in exchange_list {
            exchanges.insert(exchange.id(), exchange.clone());

            // Spawn a dedicated task for each exchange's price feed
            let price_tx: Sender<PriceData> = tx.clone();
            tokio::spawn(async move {
                // The exchange's subscribe_prices function loops forever
                exchange.subscribe_prices(price_tx).await;
            });
        }

        Self {
            exchanges,
            market_state: HashMap::new(),
            price_rx: rx,
            threshold,
            quantity,
            is_executing: false,
        }
    }
    /// The main event loop for the engine
    pub async fn run(&mut self) {
        println!("🚀 Arbitrage Engine is running...");
        while let Some(price_data) = self.price_rx.recv().await {
            // 1. Update the market state for the exchange that sent data
            self.market_state
                .insert(price_data.exchange, price_data.clone());

            // 2. If we're already busy placing an order, skip this tick
            if self.is_executing {
                continue;
            }

            // 3. Check for arbitrage opportunities
            self.check_for_opportunity(price_data.exchange).await;
        }
    }

    /// This function replaces your `compare_and_execute`
    async fn check_for_opportunity(&mut self, updated_exchange_id: ExchangeId) {
        // Get the snapshot for the exchange that just updated
        // Replaces: guard!(let Some(a_snapshot) = ... else { return; });
        let Some(a_snapshot) = self.market_state.get(&updated_exchange_id) else {
            return; // No data for this exchange yet, just return.
        };

        // Iterate over all *other* exchanges in our state
        for (b_exchange_id, b_snapshot) in &self.market_state {
            if *b_exchange_id == updated_exchange_id {
                continue; // Don't compare with self
            }

            // --- ARBITRAGE CHECK ---
            // Opportunity 1: Buy on A, Sell on B
            let diff_ab = (b_snapshot.bid - a_snapshot.ask) / a_snapshot.ask;

            if diff_ab > self.threshold {
                println!(
                    "📈 OPPORTUNITY ({}): BUY {:.5} @ {} | SELL {:.5} @ {}",
                    a_snapshot.symbol,
                    a_snapshot.exchange,
                    a_snapshot.ask,
                    b_snapshot.exchange,
                    b_snapshot.bid,
                );

                self.execute_trade(
                    updated_exchange_id,
                    *b_exchange_id,
                    a_snapshot.ask,
                    b_snapshot.bid,
                )
                .await;
                return; // Stop checking after finding one
            }

            // Opportunity 2: Buy on B, Sell on A
            let diff_ba = (a_snapshot.bid - b_snapshot.ask) / b_snapshot.ask;

            if diff_ba > self.threshold {
                println!(
                    "📈 OPPORTUNITY ({}): BUY {:.5} @ {} | SELL {:.5} @ {}",
                    a_snapshot.symbol,
                    b_snapshot.exchange,
                    b_snapshot.ask,
                    a_snapshot.exchange,
                    a_snapshot.bid,
                );

                self.execute_trade(
                    *b_exchange_id,
                    updated_exchange_id,
                    b_snapshot.ask,
                    a_snapshot.bid,
                )
                .await;
                return; // Stop checking after finding one
            }
        }
    }

    /// Executes the buy and sell orders concurrently
    async fn execute_trade(
        &mut self,
        buy_exchange_id: ExchangeId,
        sell_exchange_id: ExchangeId,
        buy_price: f64,
        sell_price: f64,
    ) {
        self.is_executing = true; // Lock the engine

        let Some(buy_exchange) = self.exchanges.get(&buy_exchange_id) else {
            eprintln!("Error: Buy exchange not found");
            self.is_executing = false;
            return;
        };

        let Some(sell_exchange) = self.exchanges.get(&sell_exchange_id) else {
            eprintln!("Error: Sell exchange not found");
            self.is_executing = false;
            return;
        };

        println!("--- EXECUTION ---");
        let buy_future = buy_exchange.place_order_future(OrderSide::Buy, buy_price, self.quantity);
        let sell_future =
            sell_exchange.place_order_future(OrderSide::Sell, sell_price, self.quantity);

        match tokio::try_join!(buy_future, sell_future) {
            Ok((buy_id, sell_id)) => {
                println!("✅✅✅ TRADE EXECUTED ✅✅✅");
                println!("  -> BUY ID:  {}", buy_id);
                println!("  -> SELL ID: {}", sell_id);
            }
            Err(e) => {
                eprintln!("❌❌❌ TRADE FAILED: {:?} ❌❌❌", e);
                eprintln!("!!! CRITICAL: Check for partial fills!");
            }
        }
        println!("-----------------");

        time::sleep(Duration::from_secs(5)).await;
        self.is_executing = false; // Unlock the engine
    }
}
