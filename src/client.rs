use anyhow::{Result, anyhow};
use chrono::Duration;
use dashmap::DashMap;
use http::HeaderValue;
use reqwest::header;
use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;
use std::time;
use tokio::sync::{Mutex, RwLock, Semaphore};
use url::Url;

pub struct HttpClient {
    client: reqwest::Client,
    client_auto_redirect: reqwest::Client,
    sem_per_host: Arc<DashMap<String, Arc<Semaphore>>>,
    sem_global: Arc<Semaphore>,
    total_requests_made: Mutex<u64>,
    total_retry: Mutex<u64>,
    total_failed: Mutex<u64>,
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
        // MBSW_IS_HUMAN=Passed Dec-26 3:03 AM; expires=Wed, 31-Dec-2025 08:03:01 GMT; path=/; SameSite=Strict
        let mut cookie_value = String::from("MBSW_IS_HUMAN=Passed ");
        let now = chrono::Utc::now() - Duration::days(1);
        let from_dt = now.format("%b-%d %I:%M %P").to_string();
        let upto = now + Duration::days(7);
        let to_dt = upto.format("%a, %d-%b-%Y %H:%M:%S").to_string();

        cookie_value.push_str(&from_dt);
        cookie_value.push_str("; expires=");
        cookie_value.push_str(&to_dt);
        cookie_value.push_str(" GMT; path=/; SameSite=Strict");

        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(cookie_value.as_str()).unwrap(),
        );

        let c = reqwest::Client::builder()
            .timeout(time::Duration::from_secs(120))
            .connect_timeout(time::Duration::from_secs(10))
            .pool_max_idle_per_host(3)
            .pool_idle_timeout(time::Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .default_headers(headers.clone())
            .build()
            .unwrap();

        let c_auto_redirect = reqwest::Client::builder()
            .timeout(time::Duration::from_secs(120))
            .connect_timeout(time::Duration::from_secs(10))
            .pool_max_idle_per_host(3)
            .pool_idle_timeout(time::Duration::from_secs(5))
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            client: c,
            client_auto_redirect: c_auto_redirect,
            sem_per_host: Arc::new(DashMap::new()),
            sem_global: Arc::new(Semaphore::new(80)),
            total_failed: Mutex::new(0),
            total_retry: Mutex::new(0),
            total_requests_made: Mutex::new(0),
        }
    }

    pub async fn get(&self, url: &str) -> Result<Response> {
        let u = http::uri::Uri::from_str(url)?;
        let host = u.host().unwrap().to_string();

        // Get or create semaphore for this host (thread-safe)
        let sem = self
            .sem_per_host
            .entry(host)
            .or_insert_with(|| Arc::new(Semaphore::new(10)))
            .clone();

        let mut t = 0;

        loop {
            if t > 3 {
                let mut n = self.total_failed.lock().await;
                *n += 1;
                return Err(anyhow!("retry failed"));
            }
            t += 1;

            let _global_permit = self.sem_global.acquire().await?;
            let _permit = sem.acquire().await?;

            {
                let mut n = self.total_requests_made.lock().await;
                *n += 1;
            }

            if t > 1 {
                let mut n = self.total_retry.lock().await;
                *n += 1;
            }
            let r = self.client.get(url).send().await;

            let response = match r {
                Ok(s) => s,
                Err(e) => {
                    drop(_global_permit);
                    eprintln!("Request failed for URL: {}. retrying", url);
                    eprintln!("Error: {}", e);
                    eprintln!("Is timeout: {}", e.is_timeout());
                    eprintln!("Is connect: {}", e.is_connect());
                    eprintln!("Is request: {}", e.is_request());

                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
            };
            // drop(_permit);
            // drop(_global_permit);

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
            let contents = match response.text().await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to read response text for URL: {}", url);
                    eprintln!("Error: {}", e);
                    if let Some(source) = Error::source(&e) {
                        eprintln!("Source: {}", source);
                    }
                    continue;
                }
            };
            if t > 1 {
                eprintln!("retry successful for read response");
            }

            return Ok(Response::Content(contents));
        }
    }

    pub async fn get_auto_redirect(&self, url: &str) -> Result<String> {
        let u = http::uri::Uri::from_str(url)?;
        let host = u.host().unwrap().to_string();

        // println!("acquiring global");
        // let _global_permit = self.sem_global.acquire().await?;

        // println!("acquiring per host");

        // Get or create semaphore for this host (thread-safe)
        let sem = self
            .sem_per_host
            .entry(host)
            .or_insert_with(|| Arc::new(Semaphore::new(10)))
            .clone();
        let _permit = sem.acquire().await?;

        let mut t = 0;
        let response = loop {
            if t > 3 {
                let mut n = self.total_failed.lock().await;
                *n += 1;
                return Err(anyhow!("retry failed"));
            }
            t += 1;
            let _global_permit = self.sem_global.acquire().await?;
            {
                let mut n = self.total_requests_made.lock().await;
                *n += 1;
            }

            let r = self.client_auto_redirect.get(url).send().await;

            match r {
                Ok(s) => {
                    if t > 1 {
                        let mut n = self.total_requests_made.lock().await;
                        *n += 1;
                        eprintln!("retry successful");
                    }
                    break s;
                }
                Err(e) => {
                    drop(_global_permit);
                    eprintln!("Request failed for URL: {}. retrying", url);
                    eprintln!("Error: {}", e);
                    eprintln!("Is timeout: {}", e.is_timeout());
                    eprintln!("Is connect: {}", e.is_connect());
                    eprintln!("Is request: {}", e.is_request());

                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
            }
        };

        drop(_permit);

        let contents = response.text().await.map_err(|e| {
            eprintln!("Failed to read response text for URL: {}", url);
            eprintln!("Error: {}", e);
            if let Some(source) = Error::source(&e) {
                eprintln!("Source: {}", source);
            }
            e
        });

        match contents {
            Ok(c) => Ok(c),
            Err(e) => {
                let mut n = self.total_failed.lock().await;
                *n += 1;
                Err(anyhow!("{}", e))
            }
        }
    }

    pub async fn summary(&self) {
        let total_requests = self.total_requests_made.lock().await;
        let total_retry = self.total_retry.lock().await;
        let total_failed = self.total_failed.lock().await;

        eprintln!("{:-<60}", "");
        eprintln!("{:<30}: {}", "Total Requests", *total_requests);
        eprintln!("{:<30}: {}", "Total Retries", *total_retry);
        eprintln!("{:<30}: {}", "Total Failed", *total_failed);
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
            .timeout(time::Duration::from_secs(120))
            .connect_timeout(time::Duration::from_secs(30))
            .pool_max_idle_per_host(20)
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            client: c,
            sem_per_host: Arc::new(DashMap::new()),
            sem_global: Arc::new(Semaphore::new(50)),
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
