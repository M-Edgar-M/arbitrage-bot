# Arbitrage Bot

A high-performance cryptocurrency arbitrage trading bot built in Rust. It tracks market discrepancies between exchanges in real-time and executes trades to capitalize on price differences.

## Features

- **Multi-Exchange Support**: Connects to Binance (Spot/Futures) and Bybit via WebSockets.
- **Real-Time Orderbook Tracking**: Listens to live orderbook streams to analyze Bid/Ask prices and identify arbitrage opportunities.
- **Automated Order Execution**: Capable of placing limit and market orders asynchronously.
- **High Performance**: Built with Rust and Tokio for fast, concurrency-safe, asynchronous execution.

## Getting Started

1. Set up your environment variables by creating a `.env` file (ensure this is excluded from version control):
   ```env
   API_KEY_BINANCE=your_api_key
   SECRET_KEY_BINANCE=your_secret_key
   ```
2. Build and run the project:
   ```bash
   cargo run --release
   ```

## Architecture

- `src/ws/`: Handles WebSocket connections and orderbook streams for different exchanges.
- `src/binance/` & `src/models/`: Encapsulates exchange API interactions, authentication, and order placement.
- `src/models/orderbook.rs`: Contains the core logic for market tracking and price comparison.
