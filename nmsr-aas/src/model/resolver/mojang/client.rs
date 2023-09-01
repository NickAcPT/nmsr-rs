use crate::{
    config::MojankConfiguration,
    error::{MojangRequestError, MojangRequestResult},
};
use axum::BoxError;
use hyper::{Body, Method, Request, Response};
use std::sync::Arc;
use sync_wrapper::SyncWrapper;
use tokio::sync::RwLock;
use tower::{util::BoxService, Service};
use tower_http::{
    classify::{NeverClassifyEos, ServerErrorsFailureClass},
    trace::{DefaultOnFailure, ResponseBody},
};
use tracing::{instrument, Span};
use uuid::Uuid;

use super::model::GameProfile;

pub struct MojangClient {
    client: RwLock<
        SyncWrapper<
            BoxService<
                Request<Body>,
                Response<
                    ResponseBody<
                        Body,
                        NeverClassifyEos<ServerErrorsFailureClass>,
                        (),
                        (),
                        DefaultOnFailure,
                    >,
                >,
                BoxError,
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
    pub fn new(mojank: Arc<MojankConfiguration>) -> MojangRequestResult<Self> {
        Ok(MojangClient {
            client: crate::utils::http_client::create_http_client(mojank.session_server_rate_limit),
            mojank_config: mojank,
        })
    }

    #[instrument(skip(self, parent_span), parent = parent_span)]
    pub(crate) async fn do_request(
        &self,
        url: &str,
        method: Method,
        parent_span: &Span,
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
            .method(method)
            .uri(url)
            .body(Body::empty())?;

        let response = {
            let mut client = self.client.write().await;
            client
                .get_mut()
                .call(request)
                .await
                .map_err(MojangRequestError::BoxedRequestError)?
        };

        Ok(response)
    }

    pub async fn resolve_uuid_to_game_profile(
        &self,
        id: &Uuid,
    ) -> MojangRequestResult<GameProfile> {
        let url = format!(
            "{session_server}/session/minecraft/profile/{id}",
            session_server = self.mojank_config.session_server
        );

        let response = self.do_request(&url, Method::GET, &Span::current()).await?;
        let bytes = hyper::body::to_bytes(response.into_body()).await?;

        Ok(serde_json::from_slice(&bytes)?)
    }

    #[instrument(skip(self, parent_span), parent = parent_span)]
    pub async fn fetch_texture_from_mojang(
        &self,
        texture_id: &str,
        parent_span: &Span,
    ) -> MojangRequestResult<Vec<u8>> {
        let url = format!(
            "{textures_server}/texture/{texture_id}",
            textures_server = self.mojank_config.textures_server
        );

        let response = self.do_request(&url, Method::GET, &Span::current()).await?;
        let bytes = hyper::body::to_bytes(response.into_body()).await?;

        Ok(bytes.to_vec())
    }

    pub fn mojank_config(&self) -> &MojankConfiguration {
        self.mojank_config.as_ref()
    }
}
