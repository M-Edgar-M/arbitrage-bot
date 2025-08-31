use serde::Deserialize;

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
