use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::logger::CsvLogger;

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
}
#[derive(Debug, Clone)]
pub struct MarketSnapshot {
    pub exchange: String,
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub timestamp: i64,
}

impl MarketSnapshot {
    pub fn new(exchange: &str, symbol: &str, bid: f64, ask: f64) -> Self {
        let mid = (bid + ask) / 2.0;
        Self {
            exchange: exchange.to_string(),
            symbol: symbol.to_string(),
            bid,
            ask,
            mid,
            timestamp: Utc::now().timestamp(),
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
        snapshots: &[MarketSnapshot],
    ) -> Vec<(MarketSnapshot, MarketSnapshot, f64)> {
        let mut results = Vec::new();

        for (i, a) in snapshots.iter().enumerate() {
            for b in &snapshots[i + 1..] {
                // ✅ Skip if both are from the same exchange
                if a.exchange == b.exchange {
                    continue;
                }
                // calculate difference (mid of a vs ask of b)
                let diff = ((a.mid - b.ask).abs() / a.mid * 100000.0).round() / 100000.0;

                if diff > self.biggest_diff && diff >= self.threshold {
                    self.biggest_diff = diff;
                    results.push((a.clone(), b.clone(), diff));
                }
            }
        }

        results
    }
}

pub struct MarketTracker {
    data: HashMap<String, Vec<MarketSnapshot>>,
    comparator: Comparator,
    logger: CsvLogger,
}

impl MarketTracker {
    pub fn new(threshold: f64, log_path: &str) -> Self {
        Self {
            data: HashMap::new(),
            comparator: Comparator::new(threshold),
            logger: CsvLogger::new(log_path),
        }
    }

    pub fn update(&mut self, exchange: &str, symbol: &str, bid: f64, ask: f64) {
        let snapshot = MarketSnapshot::new(exchange, symbol, bid, ask);
        let entry: &mut Vec<MarketSnapshot> =
            self.data.entry(symbol.to_string()).or_insert_with(Vec::new);
        entry.push(snapshot);

        // Compare whenever we get a new update
        let results: Vec<(MarketSnapshot, MarketSnapshot, f64)> = self.comparator.compare(entry);
        for (a, b, diff) in results {
            self.logger.log(&a, &b, diff);
        }
    }
}
