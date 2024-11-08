use super::model::GameProfile;
use crate::{
    config::{
        MojankConfiguration, DEFAULT_TEXTURES_SERVER, DEFAULT_TEXTURES_SERVER_SKIN_URL_TEMPLATE,
    },
    error::{MojangRequestError, MojangRequestResult},
    model::resolver::mojang::model::UsernameToUuidResponse,
    utils::http_client::NmsrHttpClient,
};
use hyper::{body::Bytes, Method};
use std::{borrow::Cow, sync::Arc};
use tracing::Span;
use uuid::Uuid;

pub struct MojangClient {
    name_lookup_client: NmsrHttpClient,
    session_server_client: NmsrHttpClient,
    mojank_config: Arc<MojankConfiguration>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MojangClientKind {
    SessionServer,
    NameLookup,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MojangTextureRequestType {
    DefaultSkin,
    Skin,
    Cape,
}

impl MojangClient {
    pub fn new(mojank: Arc<MojankConfiguration>) -> MojangRequestResult<Self> {
        Ok(Self {
            session_server_client: NmsrHttpClient::new(
                mojank.session_server_rate_limit,
                mojank.session_server_timeout,
                mojank.session_server_retries,
                &mojank.outgoing_addresses,
            ),
            name_lookup_client: NmsrHttpClient::new(
                mojank
                    .username_resolve_rate_limit
                    .unwrap_or(mojank.session_server_rate_limit),
                mojank.session_server_timeout,
                mojank.session_server_retries,
                &mojank.outgoing_addresses,
            ),
            mojank_config: mojank,
        })
    }

    pub(crate) async fn do_request(
        &self,
        kind: MojangClientKind,
        url: &str,
        method: Method,
        parent_span: &Span,
        on_error: impl FnOnce() -> Option<MojangRequestError>,
    ) -> MojangRequestResult<Bytes> {
        let client = match kind {
            MojangClientKind::SessionServer => &self.session_server_client,
            MojangClientKind::NameLookup => &self.name_lookup_client,
        };

        client.do_request(url, method, parent_span, on_error).await
    }

    pub async fn resolve_name_to_uuid<'a>(&self, name: &'a str) -> MojangRequestResult<Uuid> {
        let url = format!(
            "{mojang_api_server}/users/profiles/minecraft/{encoded_name}",
            mojang_api_server = self.mojank_config.mojang_api_server,
            encoded_name = urlencoding::encode(name)
        );

        let bytes = self
            .do_request(
                MojangClientKind::NameLookup,
                &url,
                Method::GET,
                &Span::current(),
                || {
                    Some(MojangRequestError::NamedGameProfileNotFound(
                        name.to_owned(),
                    ))
                },
            )
            .await?;

        let result: UsernameToUuidResponse = serde_json::from_slice(&bytes)
            .map_err(|_| MojangRequestError::NamedGameProfileNotFound(name.to_owned()))?;

        Ok(result.id())
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
            .do_request(
                MojangClientKind::SessionServer,
                &url,
                Method::GET,
                &Span::current(),
                || Some(MojangRequestError::GameProfileNotFound(id.to_owned())),
            )
            .await?;

        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn fetch_texture_from_mojang(
        &self,
        texture_id: &str,
        texture_url: Option<&str>,
        req_type: MojangTextureRequestType,
    ) -> MojangRequestResult<Vec<u8>> {
        let url: Cow<str> = texture_url
            .map(|u| u.into())
            .unwrap_or_else(|| self.build_request_url(req_type, texture_id).into());

        let bytes = self
            .do_request(
                MojangClientKind::SessionServer,
                &url,
                Method::GET,
                &Span::current(),
                || {
                    Some(MojangRequestError::InvalidTextureHashError(
                        texture_id.to_string(),
                    ))
                },
            )
            .await?;

        Ok(bytes.to_vec())
    }

    pub fn mojank_config(&self) -> &MojankConfiguration {
        self.mojank_config.as_ref()
    }

    fn build_request_url(&self, req_type: MojangTextureRequestType, texture_id: &str) -> String {
        let mojank = self.mojank_config();

        let req_type_default_skin = (req_type, mojank.default_skins_use_official_textures_server);

        let url = match req_type_default_skin {
            (MojangTextureRequestType::DefaultSkin, true) => {
                DEFAULT_TEXTURES_SERVER_SKIN_URL_TEMPLATE
            }
            (MojangTextureRequestType::Skin, _) | (MojangTextureRequestType::DefaultSkin, _) => {
                &mojank.textures_server_skin_url_template
            }
            (MojangTextureRequestType::Cape, _) => &mojank.textures_server_cape_url_template,
        };

        let target_server = match req_type_default_skin {
            (MojangTextureRequestType::DefaultSkin, true) => DEFAULT_TEXTURES_SERVER,
            _ => &mojank.textures_server,
        };

        url.replace("{textures_server}", target_server)
            .replace("{texture_id}", texture_id)
    }
}
