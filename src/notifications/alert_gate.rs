//! Rate-limited, deduplicated alert gating for Telegram notifications.
//!
//! `AlertGate` ensures we only fire a Telegram message when:
//! 1. `diff_percent >= min_diff` (e.g. 5%)
//! 2. For the same pair key, the diff jumped by at least `re_alert_delta` (e.g. 1pp)
//! 3. At least `cooldown` time has passed since the last send

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use super::telegram::AppAlert;

/// Composite key for deduplication: "SYMBOL|EXCHANGE_A|EXCHANGE_B"
fn pair_key(symbol: &str, exchange_a: &str, exchange_b: &str) -> String {
    // Normalise order so (binance,bybit) == (bybit,binance)
    let (a, b) = if exchange_a <= exchange_b {
        (exchange_a, exchange_b)
    } else {
        (exchange_b, exchange_a)
    };
    format!("{}|{}|{}", symbol.to_uppercase(), a, b)
}

pub struct AlertGate {
    /// Last diff% we actually notified for each pair key.
    last_notified: HashMap<String, f64>,
    /// When the last Telegram API call was made (global).
    last_send_time: Option<Instant>,
    /// Minimum diff% required to even consider alerting.
    min_diff: f64,
    /// The new diff must exceed last notified diff by at least this many pp.
    re_alert_delta: f64,
    /// Global cooldown between any two sends.
    cooldown: Duration,
}

impl AlertGate {
    pub fn new(min_diff: f64, re_alert_delta: f64, cooldown_secs: u64) -> Self {
        Self {
            last_notified: HashMap::new(),
            last_send_time: None,
            min_diff,
            re_alert_delta,
            cooldown: Duration::from_secs(cooldown_secs),
        }
    }

    /// Evaluate all three guards and, if they pass, enqueue the alert.
    ///
    /// This is intentionally **synchronous** (`try_send`) so we never block
    /// the hot path that feeds `MarketTracker::update`.
    pub fn maybe_send(
        &mut self,
        tx: &mpsc::Sender<AppAlert>,
        symbol: &str,
        exchange_a: &str,
        exchange_b: &str,
        bid_a: f64,
        ask_a: f64,
        mid_a: f64,
        bid_b: f64,
        ask_b: f64,
        mid_b: f64,
        diff_percent: f64,
    ) {
        // ── Guard 1: minimum diff ────────────────────────────────────────
        if diff_percent < self.min_diff {
            return;
        }

        // ── Guard 2: re-alert delta ──────────────────────────────────────
        let key = pair_key(symbol, exchange_a, exchange_b);
        if let Some(&prev) = self.last_notified.get(&key) {
            if diff_percent < prev + self.re_alert_delta {
                return; // not a big enough jump
            }
        }

        // ── Guard 3: global cooldown ─────────────────────────────────────
        if let Some(last) = self.last_send_time {
            if last.elapsed() < self.cooldown {
                return; // too soon
            }
        }

        // ── All guards passed — fire it ──────────────────────────────────
        let alert = AppAlert {
            symbol: symbol.to_string(),
            exchange_a: exchange_a.to_string(),
            exchange_b: exchange_b.to_string(),
            bid_a,
            ask_a,
            mid_a,
            bid_b,
            ask_b,
            mid_b,
            diff_percent,
        };

        // Non-blocking send — if the channel is full we just drop the alert.
        match tx.try_send(alert) {
            Ok(_) => {
                self.last_notified.insert(key, diff_percent);
                self.last_send_time = Some(Instant::now());
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                eprintln!("[AlertGate] Channel full — alert dropped for {}", key);
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                eprintln!("[AlertGate] Channel closed — Telegram worker gone");
            }
        }
    }

    /// Wipe all tracked state (called by the 24-hour scheduler).
    pub fn reset(&mut self) {
        self.last_notified.clear();
        self.last_send_time = None;
        println!("[AlertGate] Notification state reset (24h scheduler)");
    }
}
