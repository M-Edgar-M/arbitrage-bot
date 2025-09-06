use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{collections::HashMap, fs::OpenOptions};

#[derive(Debug, Deserialize)]
pub struct OrderBookMsg {
    pub topic: String,
    #[serde(rename = "type")]
    pub msg_type: String,
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
}

impl Comparator {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Compare snapshots for a single symbol
    pub fn compare(
        &self,
        snapshots: &[MarketSnapshot],
    ) -> Vec<(MarketSnapshot, MarketSnapshot, f64)> {
        let mut results = Vec::new();

        for (i, a) in snapshots.iter().enumerate() {
            for b in &snapshots[i + 1..] {
                let diff = (a.mid - b.ask).abs() / a.mid;
                if diff >= self.threshold {
                    results.push((a.clone(), b.clone(), diff));
                }
            }
        }

        results
    }
}

pub struct CsvLogger {
    path: String,
}

impl CsvLogger {
    pub fn new(path: &str) -> Self {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();

        // Write header if file is empty
        use std::io::Seek;
        if file.seek(std::io::SeekFrom::End(0)).unwrap() == 0 {
            writeln!(
                file,
                "symbol,exchange_a,exchange_b,bid_a,ask_a,mid_a,bid_b,ask_b,mid_b,diff_percent,timestamp"
            ).unwrap();
        }

        Self {
            path: path.to_string(),
        }
    }

    pub fn log(&self, a: &MarketSnapshot, b: &MarketSnapshot, diff: f64) {
        let mut file = OpenOptions::new().append(true).open(&self.path).unwrap();

        writeln!(
            file,
            "{},{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}%,{}",
            a.symbol,
            a.exchange,
            b.exchange,
            a.bid,
            a.ask,
            a.mid,
            b.bid,
            b.ask,
            b.mid,
            diff * 100.0,
            a.timestamp
        )
        .unwrap();
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
        let entry = self.data.entry(symbol.to_string()).or_insert_with(Vec::new);
        entry.push(snapshot);

        // Compare whenever we get a new update
        let results = self.comparator.compare(entry);
        for (a, b, diff) in results {
            self.logger.log(&a, &b, diff);
        }
    }
}
