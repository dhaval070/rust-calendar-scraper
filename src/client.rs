use anyhow::Result;
use dashmap::DashMap;
use http::HeaderValue;
use reqwest::header;
use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;
use std::time;
use tokio::sync::{RwLock, Semaphore};
use url::Url;

pub struct HttpClient {
    client: reqwest::Client,
    client_auto_redirect: reqwest::Client,
    sem_per_host: Arc<DashMap<String, Arc<Semaphore>>>,
    sem_global: Arc<Semaphore>,
}

pub enum Response {
    Content(String),
    Redirect(String),
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
            .redirect(reqwest::redirect::Policy::none())
            .default_headers(headers.clone())
            .build()
            .unwrap();

        let c_auto_redirect = reqwest::Client::builder()
            .timeout(time::Duration::from_secs(60))
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            client: c,
            client_auto_redirect: c_auto_redirect,
            sem_per_host: Arc::new(DashMap::new()),
            sem_global: Arc::new(Semaphore::new(30)),
        }
    }

    pub async fn get(&self, url: &str) -> Result<Response> {
        let u = http::uri::Uri::from_str(url)?;
        let host = u.host().unwrap().to_string();

        // println!("acquiring global");
        let _global_permit = self.sem_global.acquire().await?;

        // println!("acquiring per host");

        // Get or create semaphore for this host (thread-safe)
        let sem = self
            .sem_per_host
            .entry(host)
            .or_insert_with(|| Arc::new(Semaphore::new(10)))
            .clone();
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
        drop(_global_permit);
        drop(_permit);

        if let Some(redirect) = response.headers().get("location") {
            let redirect: String = redirect.to_str()?.into();
            let p = redirect
                .split("?")
                .next()
                .unwrap_or_else(|| redirect.as_str());
            let mut u = Url::parse(url)?;
            u.set_path(p);
            let final_url = u.to_string();
            return Ok(Response::Redirect(final_url));
        }
        let contents = response.text().await.map_err(|e| {
            eprintln!("Failed to read response text for URL: {}", url);
            eprintln!("Error: {}", e);
            if let Some(source) = Error::source(&e) {
                eprintln!("Source: {}", source);
            }
            e
        })?;
        Ok(Response::Content(contents))
    }
    pub async fn get_auto_redirect(&self, url: &str) -> Result<String> {
        let u = http::uri::Uri::from_str(url)?;
        let host = u.host().unwrap().to_string();

        // println!("acquiring global");
        let _global_permit = self.sem_global.acquire().await?;

        // println!("acquiring per host");

        // Get or create semaphore for this host (thread-safe)
        let sem = self
            .sem_per_host
            .entry(host)
            .or_insert_with(|| Arc::new(Semaphore::new(10)))
            .clone();
        let _permit = sem.acquire().await?;

        let response = self
            .client_auto_redirect
            .get(url)
            .send()
            .await
            .map_err(|e| {
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
        drop(_global_permit);
        drop(_permit);

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

pub struct HttpClientCached {
    client: reqwest::Client,
    sem_per_host: Arc<DashMap<String, Arc<Semaphore>>>,
    sem_global: Arc<Semaphore>,
    cache: Arc<DashMap<String, Arc<RwLock<String>>>>,
}

impl HttpClientCached {
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
            sem_global: Arc::new(Semaphore::new(30)),
            cache: Arc::new(DashMap::new()),
        }
    }

    pub async fn get(&self, url: &str) -> Result<String> {
        // println!("acquiring lock");
        let s = self
            .cache
            .entry(url.into())
            .or_insert(Arc::new(RwLock::new("".into())))
            .clone();

        let guard = s.read().await;

        if *guard != "" {
            eprintln!("cache hit ");
            return Ok((*guard).to_string());
        }
        drop(guard);
        let mut guard = s.write().await;

        if *guard != "" {
            eprintln!("cache hit ");
            return Ok((*guard).to_string());
        }

        let u = http::uri::Uri::from_str(url)?;
        let host = u.host().unwrap().to_string();

        // println!("acquiring global");
        let _global_permit = self.sem_global.acquire().await?;

        // println!("acquiring per host");

        // Get or create semaphore for this host (thread-safe)
        let sem = self
            .sem_per_host
            .entry(host)
            .or_insert_with(|| Arc::new(Semaphore::new(3)))
            .clone();
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
        drop(_global_permit);
        drop(_permit);
        let contents = response.text().await.map_err(|e| {
            eprintln!("Failed to read response text for URL: {}", url);
            eprintln!("Error: {}", e);
            if let Some(source) = Error::source(&e) {
                eprintln!("Source: {}", source);
            }
            e
        })?;
        *guard = contents.clone();
        Ok(contents)
    }
}
