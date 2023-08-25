use hyper::{client::HttpConnector, Body, Client, Method, Request, Response};
use hyper_tls::HttpsConnector;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower_http::{
    classify::{
        NeverClassifyEos, ServerErrorsAsFailures, ServerErrorsFailureClass, SharedClassifier,
    },
    trace::{
        DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, ResponseBody,
        Trace, TraceLayer,
    },
};
use uuid::Uuid;

use tower::{limit::RateLimit, Service, ServiceBuilder};

use crate::{
    config::MojankConfiguration,
    error::{MojangRequestError, MojangRequestResult},
};

use super::model::GameProfile;

pub struct MojangClient {
    client: RwLock<
        RateLimit<
            Trace<
                Client<HttpsConnector<HttpConnector>, Body>,
                SharedClassifier<ServerErrorsAsFailures>,
                DefaultMakeSpan,
                DefaultOnRequest,
                DefaultOnResponse,
                (),
                (),
                DefaultOnFailure,
            >,
        >,
    >,
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
        mojank: Arc<MojankConfiguration>,
    ) -> MojangRequestResult<Self> {
        let https = HttpsConnector::new();

        let client = Client::builder().build(https);

        let tracing = TraceLayer::new_for_http().on_body_chunk(()).on_eos(());
        let service = ServiceBuilder::new()
            .rate_limit(mojank.session_server_rate_limit, Duration::from_secs(1))
            .layer(tracing)
            .service(client);

        Ok(MojangClient {
            client: RwLock::new(service),
            mojank_config: mojank,
        })
    }

    async fn do_request(
        &self,
        url: &str,
        method: Method,
    ) -> MojangRequestResult<
        Response<
            ResponseBody<
                Body,
                NeverClassifyEos<ServerErrorsFailureClass>,
                (),
                (),
                DefaultOnFailure,
            >,
        >,
    > {
        let request = Request::builder()
            .header("user-agent", Self::USER_AGENT)
            .method(method)
            .uri(url)
            .body(Body::empty())?;

        let response = {
            let mut client = self.client.write().await;
            client.call(request).await?
        };

        if response.status().is_client_error() || response.status().is_server_error() {
            let status = &response.status();
            let reason = status
                .canonical_reason()
                .unwrap_or(status.as_str())
                .to_string();
            return Err(MojangRequestError::MojangRequestError(reason));
        }

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
        let bytes = hyper::body::to_bytes(response.into_body()).await?;

        Ok(serde_json::from_slice(&bytes)?)
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
        let bytes = hyper::body::to_bytes(response.into_body()).await?;

        Ok(bytes.to_vec())
    }
}
