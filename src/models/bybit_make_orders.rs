use hmac::{Hmac, Mac};
use serde::Serialize;
use serde_json::json;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub struct BybitAuth {
    api_key: String,
    secret: String,
}

impl BybitAuth {
    pub fn new(api_key: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            secret: secret.into(),
        }
    }

    /// Generate 'expires' timestamp (in ms)
    pub fn expires(&self) -> i64 {
        // e.g. current timestamp + a small offset (like 1000 ms)
        // Be careful: docs say expires must be > current time
        let now = chrono::Utc::now().timestamp_millis();
        now + 1000
    }

    /// Generate signature per Bybit: sign with HMAC SHA256 over some message
    /// The message might be "GET/realtime{expires}" or something defined in docs
    pub fn sign(&self, expires: i64) -> String {
        let payload = format!("GET/realtime{}", expires);
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes()).unwrap();
        mac.update(payload.as_bytes());
        let result = mac.finalize().into_bytes();
        hex::encode(result)
    }
}

#[derive(Serialize)]
struct BybitOrderCreateArgs {
    category: String, // "linear", "spot", "inverse"
    symbol: String,   // e.g. "BTCUSDT"
    side: String,     // "Buy" or "Sell"
    #[serde(rename = "orderType")]
    order_type: String, // "Market" or "Limit"
    qty: String,      // must be string per API docs
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<String>, // required if Limit
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "timeInForce")]
    time_in_force: Option<String>, // e.g. "GTC"
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "reduceOnly")]
    reduce_only: Option<bool>,
}

#[derive(serde::Serialize)]
struct BybitAuthMsg {
    op: String,                   // "auth"
    args: Vec<serde_json::Value>, // [apiKey, expires, signature]
}

impl BybitAuth {
    pub fn auth_msg(&self) -> BybitAuthMsg {
        let expires = self.expires();
        let sig = self.sign(expires);
        BybitAuthMsg {
            op: "auth".into(),
            args: vec![json!(self.api_key), json!(expires), json!(sig)],
        }
    }
}
