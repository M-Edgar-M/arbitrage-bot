use crate::models::orderbook::{BinanceOrderBookMsg, MarketSnapshot, OrderBookMsg};
use std::fs::OpenOptions;
use std::io::Write;

pub fn _log_orderbook(msg: &OrderBookMsg) {
    if let (Some(bid), Some(ask)) = (msg.data.b.get(0), msg.data.a.get(0)) {
        let bid_price: f64 = bid[0].parse().unwrap_or(0.0);
        let bid_size: f64 = bid[1].parse().unwrap_or(0.0);
        let ask_price: f64 = ask[0].parse().unwrap_or(0.0);
        let ask_size: f64 = ask[1].parse().unwrap_or(0.0);

        let mid_price = (bid_price + ask_price) / 2.0;

        println!(
            "📊 {} | Bid: {:.2} ({:.4}) | Ask: {:.2} ({:.4}) | Mid: {:.2} | Seq: {}",
            msg.data.s, bid_price, bid_size, ask_price, ask_size, mid_price, msg.data.seq
        );
    }
}

pub fn _log_binance_orderbook(msg: &BinanceOrderBookMsg) {
    if let (Some(bid), Some(ask)) = (msg.bids.get(0), msg.asks.get(0)) {
        let bid_price: f64 = bid[0].parse().unwrap_or(0.0);
        let bid_size: f64 = bid[1].parse().unwrap_or(0.0);
        let ask_price: f64 = ask[0].parse().unwrap_or(0.0);
        let ask_size: f64 = ask[1].parse().unwrap_or(0.0);

        let mid_price = (bid_price + ask_price) / 2.0;

        println!(
            "📊 {} | Bid: {:.2} ({:.4}) | Ask: {:.2} ({:.4}) | Mid: {:.2}",
            msg.symbol, bid_price, bid_size, ask_price, ask_size, mid_price
        );
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

        let line = format!(
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
        );

        writeln!(file, "{}", line).unwrap();

        // also print it to console
        // println!("After write file::{}", line);
    }
}
