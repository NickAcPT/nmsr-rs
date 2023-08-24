use reqwest::{Client, Method, Request, Response, Url};
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use tower::{limit::RateLimit, Service, ServiceBuilder};

use crate::{config::MojankConfiguration, error::MojangRequestResult};

use super::model::GameProfile;

pub struct MojangClient {
    client: RwLock<RateLimit<reqwest::Client>>,
    mojank_config: Arc<MojankConfiguration>,
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

    pub fn new(
        rate_limit_per_second: u64,
        mojank: Arc<MojankConfiguration>,
    ) -> MojangRequestResult<Self> {
        let client = Client::builder().user_agent(Self::USER_AGENT).build()?;

        let tracing = TraceLayer::new_for_http();//.on_body_chunk(()).on_eos(());
        let service = ServiceBuilder::new()
            .rate_limit(rate_limit_per_second, Duration::from_secs(1))
            .service(client);

        Ok(MojangClient {
            client: RwLock::new(service),
            mojank_config: mojank,
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

    pub async fn resolve_uuid_to_game_profile(
        &self,
        id: &Uuid,
    ) -> MojangRequestResult<GameProfile> {
        let url = format!(
            "{session_server}/{id}",
            session_server = self.mojank_config.session_server
        );

        let response = self.do_request(&url, Method::GET).await?;

        Ok(response.json::<GameProfile>().await?)
    }

    pub async fn fetch_texture_from_mojang(
        &self,
        texture_id: &str,
    ) -> MojangRequestResult<Vec<u8>> {
        let url = format!(
            "{textures_server}/textures/{texture_id}",
            textures_server = self.mojank_config.textures_server
        );

        let response = self.do_request(&url, Method::GET).await?;

        Ok(response.bytes().await?.to_vec())
    }
}
