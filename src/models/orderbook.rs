use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::{
    logger::CsvLogger,
    notifications::{alert_gate::AlertGate, telegram::AppAlert},
};

#[derive(Debug, Deserialize)]
pub struct OrderBookMsg {
    pub topic: String,
    #[serde(rename = "type")]
    pub _msg_type: String,
    pub data: OrderBookData,
}

#[derive(Debug, Deserialize)]
pub struct OrderBookData {
    pub s: String,           // symbol
    pub b: Vec<[String; 2]>, // bids [price, size]
    pub a: Vec<[String; 2]>, // asks [price, size]
    pub u: u64,              // update ID
    pub seq: u64,            // sequence
    #[serde(skip)]
    pub market_type: MarketType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BinanceOrderBookMsg {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub bids: Vec<Vec<String>>,
    #[serde(rename = "a")]
    pub asks: Vec<Vec<String>>,
    #[serde(skip)]
    pub market_type: MarketType,
}

// Futures struct
#[derive(Debug, Deserialize)]
pub struct BinanceFuturesOrderBookMsg {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "U")]
    pub first_update_id: u64,
    #[serde(rename = "u")]
    pub final_update_id: u64,
    #[serde(rename = "pu")]
    pub prev_final_update_id: Option<u64>,
    #[serde(rename = "b")]
    pub bids: Vec<Vec<String>>,
    #[serde(rename = "a")]
    pub asks: Vec<Vec<String>>,
    #[serde(skip)]
    pub market_type: MarketType,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum MarketType {
    #[default] // required for Default trait
    Spot,
    Futures,
}

#[derive(Debug, Deserialize)]
pub enum BinanceDepthUpdate {
    Spot(BinanceOrderBookMsg),
    Futures(BinanceFuturesOrderBookMsg),
}

#[derive(Debug, Clone)]
pub struct MarketSnapshot {
    pub exchange: String,
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub timestamp: i64,
    // DETERMINE WHETHER WE NEED THIS OR NOT
    // market_type: MarketType,
}

impl MarketSnapshot {
    pub fn new(exchange: &str, symbol: &str, bid: f64, ask: f64, market_type: MarketType) -> Self {
        let mid = (bid + ask) / 2.0;
        Self {
            exchange: exchange.to_string(),
            symbol: symbol.to_string(),
            bid,
            ask,
            mid,
            timestamp: Utc::now().timestamp(),
            // market_type,
        }
    }
}

pub struct Comparator {
    pub threshold: f64, // e.g., 0.1 = 10%
    pub biggest_diff: f64,
}

impl Comparator {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            biggest_diff: 0.0,
        }
    }

    /// Compare snapshots only across *different exchanges*
    pub fn compare(
        &mut self,
        snapshots: &HashMap<String, MarketSnapshot>,
    ) -> Vec<(MarketSnapshot, MarketSnapshot, f64)> {
        let mut results = Vec::new();
        let exchanges: Vec<&String> = snapshots.keys().collect();

        for (i, exchange_a) in exchanges.iter().enumerate() {
            for exchange_b in &exchanges[i + 1..] {
                let a = snapshots.get(*exchange_a).unwrap();
                let b = snapshots.get(*exchange_b).unwrap();

                // calculate difference (mid vs mid)
                // Formula: |a - b| / ((a + b) / 2) * 100 ? No, standard is |a - b| / min(a,b) or just one of them.
                // User's original code was: (a.mid - b.ask).abs() / a.mid
                // Let's standardise to: abs(a.mid - b.mid) / a.mid
                // But generally for arbitrage, we want (Bid_A - Ask_B) / Ask_B if we buy on B sell on A.
                // However user asked just for "price difference".
                // Let's stick closer to "spread":

                let diff = ((a.mid - b.mid).abs() / a.mid * 100.0);

                if diff >= self.threshold {
                    // Only update biggest_diff if it's actually bigger
                    if diff > self.biggest_diff {
                        self.biggest_diff = diff;
                    }
                    results.push((a.clone(), b.clone(), diff));
                }
            }
        }

        results
    }
}

pub struct MarketTracker {
    // Symbol -> Exchange -> Snapshot
    data: HashMap<String, HashMap<String, MarketSnapshot>>,
    comparator: Comparator,
    logger: CsvLogger,
    pub alert_gate: AlertGate,
    telegram_tx: Option<mpsc::Sender<AppAlert>>,
}

impl MarketTracker {
    pub fn new(
        threshold: f64,
        log_path: &str,
        telegram_tx: Option<mpsc::Sender<AppAlert>>,
        alert_gate: AlertGate,
    ) -> Self {
        Self {
            data: HashMap::new(),
            comparator: Comparator::new(threshold),
            logger: CsvLogger::new(log_path),
            alert_gate,
            telegram_tx,
        }
    }

    pub fn update(
        &mut self,
        exchange: &str,
        symbol: &str,
        bid: f64,
        ask: f64,
        _market_type: MarketType,
    ) {
        let snapshot = MarketSnapshot::new(exchange, symbol, bid, ask, _market_type);

        let symbol_entry = self
            .data
            .entry(symbol.to_string())
            .or_insert_with(HashMap::new);

        // Insert or overwrite the snapshot for this exchange
        symbol_entry.insert(exchange.to_string(), snapshot);

        // Compare using the updated map for this symbol
        let results = self.comparator.compare(symbol_entry);
        // CSV logging disabled — using Telegram notifications instead
        // for (a, b, diff) in &results {
        //     self.logger.log(a, b, *diff);
        // }

        // ── Telegram alerts ──────────────────────────────────────────
        if let Some(ref tx) = self.telegram_tx {
            for (a, b, diff) in results {
                self.alert_gate.maybe_send(
                    tx,
                    &a.symbol,
                    &a.exchange,
                    &b.exchange,
                    a.bid,
                    a.ask,
                    a.mid,
                    b.bid,
                    b.ask,
                    b.mid,
                    diff,
                );
            }
        }
    }
}
