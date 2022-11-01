use crate::mojang::caching::MojangCacheManager;
use crate::mojang::requests;
use crate::utils::errors::NMSRaaSError;
use crate::utils::Result;
use actix_web::web::Bytes;

#[derive(Debug, Clone)]
pub(crate) enum PlayerRenderInput {
    PlayerUuid(uuid::Uuid),
    TextureHash(String),
}

impl TryFrom<String> for PlayerRenderInput {
    type Error = NMSRaaSError;

    fn try_from(value: String) -> Result<PlayerRenderInput> {
        if value.len() == 32 || value.len() == 36 {
            let uuid = uuid::Uuid::parse_str(&value).map_err(NMSRaaSError::InvalidUUID)?;
            Ok(PlayerRenderInput::PlayerUuid(uuid))
        } else if value.len() > 36 {
            Ok(PlayerRenderInput::TextureHash(value))
        } else {
            Err(NMSRaaSError::InvalidPlayerRequest(value))
        }
    }
}

impl PlayerRenderInput {
    pub(crate) async fn fetch_skin_bytes(
        &self,
        cache_manager: &MojangCacheManager,
        client: &reqwest::Client,
    ) -> Result<(String, Bytes)> {
        let hash = match self {
            PlayerRenderInput::PlayerUuid(id) => requests::get_skin_hash(client, *id).await?,
            PlayerRenderInput::TextureHash(hash) => hash.to_owned(),
        };

        let result = cache_manager.get_cached_skin(&hash)?;

        if let Some(bytes) = result {
            Ok((hash, Bytes::from(bytes)))
        } else {
            let bytes_from_mojang = requests::fetch_skin_bytes_from_mojang(&hash).await?;
            cache_manager.cache_skin(&hash, &bytes_from_mojang)?;
            Ok((hash, bytes_from_mojang))
        }
    }
}
