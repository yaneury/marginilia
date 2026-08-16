use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub body: String,
    pub author: String,
    pub work: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuotesResponse {
    pub latest_timestamp: u64,
    pub quotes: Vec<Quote>,
}

pub struct QuoteStore {
    quotes: Vec<Quote>,
    latest_timestamp: u64,
}

impl QuoteStore {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let quotes: Vec<Quote> = serde_yaml::from_str(&content)?;
        let latest_timestamp = quotes.iter().map(|q| q.created_at).max().unwrap_or(0);
        Ok(Self { quotes, latest_timestamp })
    }

    pub fn len(&self) -> usize {
        self.quotes.len()
    }

    pub fn as_response(&self, since: u64) -> QuotesResponse {
        let quotes = if since == 0 {
            self.quotes.clone()
        } else {
            self.quotes.iter().filter(|q| q.created_at > since).cloned().collect()
        };
        QuotesResponse {
            latest_timestamp: self.latest_timestamp,
            quotes,
        }
    }
}
