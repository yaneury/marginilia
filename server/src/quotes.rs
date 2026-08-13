use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub body: String,
    pub author: String,
    pub work: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuotesResponse {
    pub checksum: String,
    pub quotes: Vec<Quote>,
}

pub struct QuoteStore {
    quotes: Vec<Quote>,
    checksum: String,
}

impl QuoteStore {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let quotes: Vec<Quote> = serde_yaml::from_str(&content)?;

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let checksum = hex::encode(hasher.finalize());

        Ok(Self { quotes, checksum })
    }

    pub fn len(&self) -> usize {
        self.quotes.len()
    }

    pub fn as_response(&self) -> QuotesResponse {
        QuotesResponse {
            checksum: self.checksum.clone(),
            quotes: self.quotes.clone(),
        }
    }
}
