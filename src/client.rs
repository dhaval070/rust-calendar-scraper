use anyhow::Result;
use dashmap::DashMap;
use http::HeaderValue;
use reqwest::header;
use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;
use std::time;
use tokio::sync::Semaphore;

pub struct HttpClient {
    client: reqwest::Client,
    sem_per_host: Arc<DashMap<String, Arc<Semaphore>>>,
    sem_global: Arc<Semaphore>,
}

impl HttpClient {
    pub fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_str(
                "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/112.0",
            )
            .unwrap(),
        );

        let c = reqwest::Client::builder()
            .timeout(time::Duration::from_secs(60))
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            client: c,
            sem_per_host: Arc::new(DashMap::new()),
            sem_global: Arc::new(Semaphore::new(60)),
        }
    }

    pub async fn get(&self, url: &str) -> Result<String> {
        let u = http::uri::Uri::from_str(url)?;
        let host = u.host().unwrap().to_string();

        // Get or create semaphore for this host (thread-safe)
        let sem = self
            .sem_per_host
            .entry(host)
            .or_insert_with(|| Arc::new(Semaphore::new(3)))
            .clone();

        let _global_permit = self.sem_global.acquire().await?;
        let _permit = sem.acquire().await?;

        let response = self.client.get(url).send().await.map_err(|e| {
            eprintln!("Request failed for URL: {}", url);
            eprintln!("Error: {}", e);
            eprintln!("Is timeout: {}", e.is_timeout());
            eprintln!("Is connect: {}", e.is_connect());
            eprintln!("Is request: {}", e.is_request());
            if let Some(status) = e.status() {
                eprintln!("Status code: {}", status);
            }
            if let Some(source) = Error::source(&e) {
                eprintln!("Source: {}", source);
                let mut src = source;
                while let Some(next) = Error::source(src) {
                    eprintln!("  Caused by: {}", next);
                    src = next;
                }
            }
            e
        })?;
        let contents = response.text().await.map_err(|e| {
            eprintln!("Failed to read response text for URL: {}", url);
            eprintln!("Error: {}", e);
            if let Some(source) = Error::source(&e) {
                eprintln!("Source: {}", source);
            }
            e
        })?;
        Ok(contents)
    }
}
