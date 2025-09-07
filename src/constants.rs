pub mod pairs {
    pub const BTC_USDT_BINANCE: &str = "btcusdt";
    pub const BTC_USDT_BYBIT: &str = "BTCUSDT";
    pub const ETH_USDT_BINANCE: &str = "ethusdt";
    pub const ETH_USDT_BYBIT: &str = "ETHUSDT";
}

pub mod exchange_names {
    pub const BINANCE: &str = "binance";
    pub const BYBIT: &str = "bybit";
}

pub mod thresholds {
    pub const HIGHT_THRESHOLD_10_PERCENT: f64 = 0.1;
    pub const MID_THRESHOLD_5_PERCENT: f64 = 0.05;
    pub const LOW_THRESHOLD_2_PERCENT: f64 = 0.02;
    pub const LOW_THRESHOLD_1_PERCENT: f64 = 0.01;
}
