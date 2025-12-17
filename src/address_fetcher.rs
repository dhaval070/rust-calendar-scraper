use crate::client::HttpClient;
use dashmap::DashMap;
use std::sync::Arc;

pub struct AddressFetcher {
    client: HttpClient,
    addresses: Arc<DashMap<String, Address>>,
}

struct Address {
    status: AddressStatus,
    address: String,
}

impl Address {
    fn read(&self) -> String {
        return self.address.clone();
    }
}

enum AddressStatus {
    InFlight,
    Ready,
}

impl AddressFetcher {
    pub fn new(client: HttpClient) -> Self {
        Self {
            client: client,
            addresses: Arc::new(DashMap::new()),
        }
    }

    pub fn get_address(&self, site: String, url: &str) {
        let addr = self.addresses.entry(site).or_insert_with(|| {
            let (tx, rx) = tokio::sync::watch::channel(url);
            Address {
                status: AddressStatus::InFlight,
                address: "".into(),
            }
        });
    }
}
