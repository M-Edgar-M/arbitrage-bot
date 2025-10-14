use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

pub struct BinanceAuth {
    api_key: String,
    api_secret: String,
}

impl BinanceAuth {
    pub fn new(api_key: String, api_secret: String) -> Self {
        if api_secret == "YOUR_API_SECRET_HERE" || api_key == "YOUR_API_KEY_HERE" {
            eprintln!("FATAL: Please set valid API_KEY and API_SECRET.");
            // In a real application, you might use an error type here.
            // For this example, we proceed but the API call will likely fail.
        }
        BinanceAuth {
            api_key,
            api_secret,
        }
    }

    pub fn api_key(&self) -> &String {
        &self.api_key
    }

    pub fn api_secret(&self) -> &String {
        &self.api_secret
    }

    /// Signs the query string using HMAC SHA256 and the API secret.
    ///
    /// # Arguments
    /// * `query` - The query string containing all request parameters.
    ///
    /// # Returns
    /// The HMAC SHA256 signature as a hex string.
    pub fn sign_payload(&self, query: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC SHA256 can be initialized");
        mac.update(query.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    /// Augments the request parameters with authentication details and generates the signature.
    ///
    /// This function handles adding the `apiKey`, `timestamp`, and `recvWindow`,
    /// sorting parameters, generating the query string, and signing it.
    ///
    /// # Arguments
    /// * `params` - The base parameters for the request (e.g., symbol, side, price).
    ///
    /// # Returns
    /// A BTreeMap containing all signed parameters, including `apiKey`, `timestamp`, `recvWindow`, and `signature`.
    pub fn augment_and_sign_params(
        &self,
        mut params: BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        // Get current timestamp in milliseconds
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            .to_string();

        // 1. Add mandatory authentication parameters
        params.insert("apiKey".to_string(), self.api_key.clone());
        params.insert("timestamp".to_string(), timestamp);
        params.insert("recvWindow".to_string(), 5000.to_string()); // Default 5000ms

        // 2. Build the query string by sorting keys alphabetically
        // This is crucial for Binance signature validation
        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // 3. Generate signature
        let signature = self.sign_payload(&query_string);

        // 4. Add signature to the parameter map
        params.insert("signature".to_string(), signature);

        params
    }
}
