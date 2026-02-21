//! Telegram notification service for arbitrage alerts.
//!
//! # Architecture
//! A dedicated Tokio task owns the [`TelegramNotifier`] and drains an `mpsc` channel
//! of [`AppAlert`] messages, keeping the main application loop completely non-blocking.
//!
//! # Usage
//! ```no_run
//! use crate::notifications::telegram::{TelegramNotifier, AppAlert};
//!
//! #[tokio::main]
//! async fn main() {
//!     if let Some(tx) = TelegramNotifier::spawn() {
//!         let _ = tx.try_send(AppAlert {
//!             symbol: "BTCUSDT".into(),
//!             exchange_a: "binance".into(),
//!             exchange_b: "bybit".into(),
//!             bid_a: 100_000.0, ask_a: 100_010.0, mid_a: 100_005.0,
//!             bid_b: 94_000.0,  ask_b: 94_010.0,  mid_b: 94_005.0,
//!             diff_percent: 6.38,
//!         });
//!     }
//! }
//! ```

use log::{error, info, warn};
use serde::Serialize;
use std::env;
use tokio::sync::mpsc;

// ── Public Message Type ──────────────────────────────────────────────────────

/// Arbitrage alert payload sent over the notification channel.
#[derive(Debug, Clone)]
pub struct AppAlert {
    pub symbol: String,
    pub exchange_a: String,
    pub exchange_b: String,
    pub bid_a: f64,
    pub ask_a: f64,
    pub mid_a: f64,
    pub bid_b: f64,
    pub ask_b: f64,
    pub mid_b: f64,
    pub diff_percent: f64,
}

// ── Telegram API Payload ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct SendMessagePayload<'a> {
    chat_id: &'a str,
    text: &'a str,
    parse_mode: &'a str,
    disable_notification: bool,
}

// ── Notifier ─────────────────────────────────────────────────────────────────

pub struct TelegramNotifier {
    client: reqwest::Client,
    bot_token: String,
    chat_id: String,
}

impl TelegramNotifier {
    /// Spawns the background Telegram worker.
    ///
    /// Returns `None` (with a warning log) when env vars are missing.
    pub fn spawn() -> Option<mpsc::Sender<AppAlert>> {
        let bot_token = match env::var("TELEGRAM_KEY") {
            Ok(t) if !t.is_empty() => t,
            _ => {
                warn!("TELEGRAM_KEY missing — Telegram notifications disabled.");
                return None;
            }
        };

        let chat_id = match env::var("TELEGRAM_CHAT_ID") {
            Ok(id) if !id.is_empty() => id,
            _ => {
                warn!("TELEGRAM_CHAT_ID missing — Telegram notifications disabled.");
                return None;
            }
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let notifier = Self {
            client,
            bot_token,
            chat_id,
        };

        let (tx, mut rx) = mpsc::channel::<AppAlert>(100);

        tokio::spawn(async move {
            info!("[Telegram] Worker started.");
            while let Some(alert) = rx.recv().await {
                notifier.send_message(&alert).await;
            }
            info!("[Telegram] Worker stopped.");
        });

        Some(tx)
    }

    async fn send_message(&self, alert: &AppAlert) {
        let text = format!(
            "🚨 <b>Arbitrage Alert</b>\n\n\
             📌 <b>Symbol:</b>  <code>{symbol}</code>\n\
             🏦 <b>Exchanges:</b> <code>{exch_a}</code> ↔ <code>{exch_b}</code>\n\n\
             💹 <b>{exch_a}:</b>  bid <code>{bid_a:.4}</code>  ask <code>{ask_a:.4}</code>  mid <code>{mid_a:.4}</code>\n\
             💹 <b>{exch_b}:</b>  bid <code>{bid_b:.4}</code>  ask <code>{ask_b:.4}</code>  mid <code>{mid_b:.4}</code>\n\n\
             📊 <b>Diff:</b>  <code>{diff:.2}%</code>",
            symbol = alert.symbol,
            exch_a = alert.exchange_a,
            exch_b = alert.exchange_b,
            bid_a = alert.bid_a,
            ask_a = alert.ask_a,
            mid_a = alert.mid_a,
            bid_b = alert.bid_b,
            ask_b = alert.ask_b,
            mid_b = alert.mid_b,
            diff = alert.diff_percent,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);

        let payload = SendMessagePayload {
            chat_id: &self.chat_id,
            text: &text,
            parse_mode: "HTML",
            disable_notification: false,
        };

        match self.client.post(&url).json(&payload).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!(
                    "[Telegram] Alert sent: {} ({} ↔ {}) {:.2}%",
                    alert.symbol, alert.exchange_a, alert.exchange_b, alert.diff_percent
                );
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                error!("[Telegram] API error ({}): {}", status, body);
            }
            Err(e) => {
                error!("[Telegram] Network error: {}", e);
            }
        }
    }
}
