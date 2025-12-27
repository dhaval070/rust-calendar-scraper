# rust-calendar-scraper

A small Rust-based web scraper for calendar pages (originally used to scrape hockey schedules and related HTML files).

This repository contains utilities and two HTTP clients (simple and cached) that use reqwest to fetch pages and reuse connections via a connection pool.

Date: 2025-12-27T17:09:17.692Z

## Features

- Concurrent scraping with per-host and global concurrency limits
- Connection pooling and keep-alive via reqwest (pool_max_idle_per_host, pool_idle_timeout)
- Cached client that stores fetched pages in-memory
- Config-driven behavior (see config.yaml)

## Prerequisites

- Rust (stable toolchain)
- Cargo

## Build

```bash
cargo build --release
```

## Run

Basic run (adjust as needed for your project entrypoint):

```bash
cargo run --release -- <args>
```

Note: the repository includes example HTML files and a `config.yaml` used by the scraping logic; inspect or edit `config.yaml` to control runtime behavior.

## Configuration

- config.yaml: contains configuration used by the scraper. Customize concurrency limits and targets here.
- csv/: a directory present in the repo for output CSVs (used by some scripts).

## HTTP client details

The code in `src/client.rs` builds reqwest::Client instances and sets `pool_max_idle_per_host` and `pool_idle_timeout`, so it reuses TCP connections (persistent/keep-alive) via a connection pool. It does not rely on classic HTTP/1.1 pipelining (reqwest/hyper avoid pipelining due to ordering and head-of-line blocking); when the server negotiates HTTP/2, reqwest/hyper will use HTTP/2 multiplexing which allows multiple concurrent logical streams on a single connection.

## Notes

- The clients set a default User-Agent and a sample cookie header; these can be adjusted in `src/client.rs`.
- Logging and error output is printed to stderr in several places for debugging.

## Contributing

Contributions are welcome — fork, make changes, and open a pull request. Please include tests for new functionality if applicable.

## License

Check the repository for a LICENSE file; if none is present, no license is specified.

## Contact

For questions or issues, open an issue in this repository or contact the maintainer via the repository contact details.