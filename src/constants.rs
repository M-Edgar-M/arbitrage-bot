pub mod pairs {
    pub const BTC_USDT_BINANCE: &str = "btcusdt";
    pub const BTC_USDT_BYBIT: &str = "BTCUSDT";
    pub const ETH_USDT_BINANCE: &str = "ethusdt";
    pub const ETH_USDT_BYBIT: &str = "ETHUSDT";
    pub const WLFI_USDT_BINANCE: &str = "wlfiusdt";
    pub const WLFI_USDT_BYBIT: &str = "WLFIUSDT";
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

pub mod urls {
    pub const BINANCE_URL_SPOT: &str = "wss://stream.binance.com:9443/ws"; // Spot
    pub const BINANCE_URL_FUTURES: &str = "wss://fstream.binance.com/ws"; // Futures
    pub const BYBIT_URL_SPOT: &str = "wss://stream.bybit.com/v5/public/spot"; // Spot
    pub const BYBIT_URL_FUTURES: &str = "wss://stream.bybit.com/v5/public/linear";
    // Futures
}
