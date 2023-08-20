use reqwest::{Client, Method, Request, RequestBuilder, Response, Url};
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

use tower::{buffer::Buffer, limit::RateLimit, retry::budget::Budget, Service, ServiceBuilder};

use crate::error::{MojangRequestError, MojangRequestResult};

use super::model::GameProfile;

pub struct MojangClient {
    client: RwLock<RateLimit<reqwest::Client>>,
}

#[test]
fn owo() {
    println!(env!("CARGO_PKG_AUTHORS"))
}

impl MojangClient {
    const USER_AGENT: &'static str = concat!(
        "NMSR-as-a-Service/",
        env!("CARGO_PKG_VERSION"),
        " (Discord=@nickacpt; +https://nmsr.nickac.dev/)"
    );

    pub fn new(rate_limit_per_second: u64) -> MojangRequestResult<Self> {
        let client = Client::builder().user_agent(Self::USER_AGENT).build()?;

        let service = ServiceBuilder::new()
            .rate_limit(rate_limit_per_second, Duration::from_secs(1))
            .service(client);

        Ok(MojangClient {
            client: RwLock::new(service),
        })
    }

    async fn do_request(&self, url: &str, method: Method) -> MojangRequestResult<Response> {
        let request = Request::new(method, Url::parse(url)?);

        let response = {
            let mut client = self.client.write().await;
            client.call(request).await?
        }
        .error_for_status()?;

        Ok(response)
    }

    pub async fn resolve_uuid_to_game_profile(&self, session_server: String, id: Uuid) -> MojangRequestResult<GameProfile> {
        let url = format!("{session_server}/{id}");

        let response = self.do_request(&url, Method::GET).await?;

        Ok(response.json::<GameProfile>().await?)
    }
}
