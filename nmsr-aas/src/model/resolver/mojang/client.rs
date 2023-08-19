use std::time::Duration;
use uuid::Uuid;

use hyper::{client::HttpConnector, Client};
use hyper_tls::HttpsConnector;
use tower::{
    limit::{rate::Rate, RateLimit},
    retry::{budget::Budget, Retry},
};

use crate::error::Result;

pub struct MojangClient {
    client: Retry<Budget, RateLimit<Client<HttpsConnector<HttpConnector>>>>,
}

impl MojangClient {
    pub fn new(rate_limit_per_second: u64) -> Self {
        let rate = Rate::new(rate_limit_per_second, Duration::from_secs(1));
        let budget = Budget::new(Duration::from_secs(2), 2, 5.0);

        let https = HttpsConnector::new();

        let client = Client::builder().build::<_, hyper::Body>(https);
        let client = RateLimit::new(client, rate);

        let client = Retry::new(budget, client);

        Self { client }
    }

    pub fn resolve_uuid_to_game_profile(id: Uuid) -> Result<()> {
        unimplemented!()
    }
}
