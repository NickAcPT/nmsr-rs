use super::model::GameProfile;
use crate::{
    config::MojankConfiguration,
    error::{MojangRequestError, MojangRequestResult},
    utils::http_client::NmsrHttpClient,
};
use hyper::{body::Bytes, Method};
use std::sync::Arc;
use tracing::Span;
use uuid::Uuid;

pub struct MojangClient {
    client: NmsrHttpClient,
    mojank_config: Arc<MojankConfiguration>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MojangTextureRequestType {
    Skin,
    Cape,
}

impl MojangClient {
    pub fn new(mojank: Arc<MojankConfiguration>) -> MojangRequestResult<Self> {
        Ok(Self {
            client: NmsrHttpClient::new(mojank.session_server_rate_limit),
            mojank_config: mojank,
        })
    }

    pub(crate) async fn do_request(
        &self,
        url: &str,
        method: Method,
        parent_span: &Span,
        on_error: impl FnOnce() -> Option<MojangRequestError>,
    ) -> MojangRequestResult<Bytes> {
        self.client
            .do_request(url, method, parent_span, on_error)
            .await
    }

    pub async fn resolve_uuid_to_game_profile(
        &self,
        id: &Uuid,
    ) -> MojangRequestResult<GameProfile> {
        let id_str = if self.mojank_config().use_dashless_uuids {
            id.simple().to_string()
        } else {
            id.as_hyphenated().to_string()
        };
        
        let url = format!(
            "{session_server}/session/minecraft/profile/{id_str}",
            session_server = self.mojank_config.session_server
        );

        let bytes = self
            .do_request(&url, Method::GET, &Span::current(), || {
                Some(MojangRequestError::GameProfileNotFound(id.to_owned()))
            })
            .await?;

        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn fetch_texture_from_mojang(
        &self,
        texture_id: &str,
        req_type: MojangTextureRequestType
    ) -> MojangRequestResult<Vec<u8>> {
        let url = self.build_request_url(req_type, texture_id);

        let bytes = self
            .do_request(&url, Method::GET, &Span::current(), || {
                Some(MojangRequestError::InvalidTextureHashError(
                    texture_id.to_string(),
                ))
            })
            .await?;

        Ok(bytes.to_vec())
    }

    pub fn mojank_config(&self) -> &MojankConfiguration {
        self.mojank_config.as_ref()
    }

    fn build_request_url(&self, req_type: MojangTextureRequestType, texture_id: &str) -> String {
        let mojank = self.mojank_config();

        match req_type {
            MojangTextureRequestType::Skin => mojank
                .textures_server_skin_url_template
                .replace("{textures_server}", &mojank.textures_server)
                .replace("{texture_id}", texture_id),
            MojangTextureRequestType::Cape => mojank
                .textures_server_cape_url_template
                .replace("{textures_server}", &mojank.textures_server)
                .replace("{texture_id}", texture_id),
        }
    }
}
