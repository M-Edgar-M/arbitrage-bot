use crate::models::orderbook::{BinanceOrderBookMsg, OrderBookMsg};

pub fn log_orderbook(msg: &OrderBookMsg) {
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

pub fn log_binance_orderbook(msg: &BinanceOrderBookMsg) {
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
