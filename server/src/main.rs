mod quotes;
mod routes;

use axum::{Router, middleware};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use quotes::QuoteStore;

#[derive(Clone)]
pub struct AppState {
    pub quotes: Arc<QuoteStore>,
    pub api_key: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let quotes_path = std::env::var("QUOTES_PATH").unwrap_or_else(|_| "quotes.yaml".into());
    let api_key = std::env::var("API_KEY").expect("API_KEY env var must be set");
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());

    let quotes = QuoteStore::load(&quotes_path).expect("Failed to load quotes.yaml");
    info!("Loaded {} quotes from {}", quotes.len(), quotes_path);

    let state = AppState {
        quotes: Arc::new(quotes),
        api_key,
    };

    let app = routes::router(state);

    let addr = format!("0.0.0.0:{}", port);
    info!("Listening on {}", addr);
    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
