use futures_util::StreamExt;
use rand::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

// --- Configuration Constants ---
const BASE_BACKOFF_MS: u64 = 1000;
const MAX_BACKOFF_MS: u64 = 60_000;
const CONNECTION_ROTATION_DURATION: Duration = Duration::from_secs(23 * 3600); // 23 hours
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(60); // 60 seconds without message = dead
const MAX_DISCONNECTIONS_WINDOW: Duration = Duration::from_secs(300); // 5 minutes
const MAX_DISCONNECTIONS_LIMIT: usize = 10; // 10 disconnections in 5 mins -> trips circuit breaker

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Rotating,
}

#[derive(Clone)]
pub struct WsHandler {
    pub url: String,
    pub state: Arc<Mutex<ConnectionState>>,
    pub shutdown: Arc<AtomicBool>,
    pub sender: mpsc::Sender<Result<Message, String>>,
    pub disconnection_timestamps: Arc<Mutex<Vec<Instant>>>,
    pub last_heartbeat: Arc<Mutex<Instant>>,
}

impl WsHandler {
    pub fn new(url: String, sender: mpsc::Sender<Result<Message, String>>) -> Self {
        Self {
            url,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            shutdown: Arc::new(AtomicBool::new(false)),
            sender,
            disconnection_timestamps: Arc::new(Mutex::new(Vec::new())),
            last_heartbeat: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub async fn start(&self) {
        let handler = self.clone();
        tokio::spawn(async move {
            handler.connection_loop().await;
        });
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    async fn record_disconnection(&self) {
        let mut timestamps = self.disconnection_timestamps.lock().await;
        timestamps.push(Instant::now());
        // Clean up old timestamps
        timestamps.retain(|t| t.elapsed() < MAX_DISCONNECTIONS_WINDOW);
    }

    async fn check_circuit_breaker(&self) -> bool {
        let mut timestamps = self.disconnection_timestamps.lock().await;
        timestamps.retain(|t| t.elapsed() < MAX_DISCONNECTIONS_WINDOW);
        timestamps.len() >= MAX_DISCONNECTIONS_LIMIT
    }

    async fn connection_loop(&self) {
        let mut backoff_ms = BASE_BACKOFF_MS;
        let mut rotation_deadline = Instant::now() + CONNECTION_ROTATION_DURATION;

        while !self.shutdown.load(Ordering::Relaxed) {
            // 1. Check Circuit Breaker
            if self.check_circuit_breaker().await {
                eprintln!(
                    "🔥 Circuit Breaker Tripped! Too many disconnections. Waiting 5 minutes..."
                );
                time::sleep(MAX_DISCONNECTIONS_WINDOW).await;
                // Clear timestamps after waiting to reset the breaker
                self.disconnection_timestamps.lock().await.clear();
            }

            // 2. Check Connection Rotation
            if Instant::now() >= rotation_deadline {
                println!("🔄 Proactive Connection Rotation triggered.");
                *self.state.lock().await = ConnectionState::Rotating;
                // Update rotation deadline for next cycle
                rotation_deadline = Instant::now() + CONNECTION_ROTATION_DURATION;
            }

            // 3. Connect
            *self.state.lock().await = ConnectionState::Connecting;
            println!("🔌 Connecting to WebSocket: {}", self.url);

            match connect_async(&self.url).await {
                Ok((ws_stream, _)) => {
                    println!("✅ Connected to WebSocket");
                    *self.state.lock().await = ConnectionState::Connected;
                    backoff_ms = BASE_BACKOFF_MS; // Reset backoff on successful connection
                    *self.last_heartbeat.lock().await = Instant::now();

                    self.handle_stream(ws_stream).await;
                }
                Err(e) => {
                    eprintln!("❌ Connection failed: {:?}", e);
                }
            }

            // 4. Handle Disconnection / Reconnect Logic
            if self.shutdown.load(Ordering::Relaxed) {
                println!("🛑 WebSocket Handler shutting down.");
                break;
            }

            *self.state.lock().await = ConnectionState::Reconnecting;
            self.record_disconnection().await;

            // Exponential Backoff with Jitter
            let jitter: u64 = rand::thread_rng().gen_range(0..500); // jitter in ms
                                                                    // let jitter = 100;
            let sleep_duration = Duration::from_millis(backoff_ms + jitter);
            println!("⏳ Reconnecting in {:?}...", sleep_duration);
            time::sleep(sleep_duration).await;

            // Increase backoff for next attempt, capped at MAX
            backoff_ms = std::cmp::min(backoff_ms * 2, MAX_BACKOFF_MS);
        }
        *self.state.lock().await = ConnectionState::Disconnected;
    }

    async fn handle_stream(
        &self,
        ws_stream: tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) {
        let (mut _write, mut read) = ws_stream.split();

        loop {
            // Define timeouts
            let heartbeat_check = time::sleep(Duration::from_secs(5));

            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(msg)) => {
                            *self.last_heartbeat.lock().await = Instant::now();
                            match msg {
                                Message::Text(_) | Message::Binary(_) => {
                                    if let Err(_) = self.sender.send(Ok(msg)).await {
                                        eprintln!("❌ Receiver dropped, stopping WebSocket.");
                                        break;
                                    }
                                }
                                Message::Ping(_) => {
                                     // Tungstenite handles Pong automatically
                                }
                                Message::Pong(_) => {
                                     // Heartbeat updated
                                }
                                Message::Close(_) => {
                                     println!("⚠️ Server closed connection.");
                                     break;
                                }
                                _ => {}
                            }
                        }
                         Some(Err(e)) => {
                            eprintln!("❌ WebSocket error: {:?}", e);
                             let _ = self.sender.send(Err(e.to_string())).await;
                            break;
                        }
                        None => {
                             println!("⚠️ WebSocket stream ended.");
                             break;
                        }
                    }
                }
                _ = heartbeat_check => {
                     // Check heartbeat
                    let last = *self.last_heartbeat.lock().await;
                    if last.elapsed() > HEARTBEAT_TIMEOUT {
                        eprintln!("💓 Heartbeat missed! Force reconnecting...");
                        break;
                    }
                    if self.shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                }
            }
        }
    }
}
