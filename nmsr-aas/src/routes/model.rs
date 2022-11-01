use crate::mojang::caching::MojangCacheManager;
use crate::mojang::requests;
use crate::utils::errors::NMSRaaSError;
use crate::utils::Result;
use actix_web::web::Bytes;
use parking_lot::RwLock;

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
    async fn fetch_skin_hash(
        &self,
        cache_manager: &RwLock<MojangCacheManager>,
        client: &reqwest::Client,
    ) -> Result<String> {
        Ok(match self {
            PlayerRenderInput::PlayerUuid(id) => {
                let option = {
                    let mut guard = cache_manager.write();
                    guard.get_cached_uuid_to_skin_hash(id)
                };

                if let Some(cached_hash) = option {
                    cached_hash
                } else {
                    let fetched_hash = requests::get_skin_hash(client, *id).await?;
                    {
                        cache_manager
                            .write()
                            .cache_uuid_to_skin_hash(id, &fetched_hash);
                    }
                    fetched_hash
                }
            }
            PlayerRenderInput::TextureHash(hash) => hash.to_owned(),
        })
    }

    pub(crate) async fn fetch_skin_bytes(
        &self,
        cache_manager: &RwLock<MojangCacheManager>,
        client: &reqwest::Client,
    ) -> Result<(String, Bytes)> {
        let hash = self.fetch_skin_hash(cache_manager, client).await?;

        let result = cache_manager.read().get_cached_skin(&hash)?;

        if let Some(bytes) = result {
            Ok((hash, Bytes::from(bytes)))
        } else {
            let bytes_from_mojang = requests::fetch_skin_bytes_from_mojang(&hash).await?;
            {
                cache_manager
                    .write()
                    .cache_skin(&hash, &bytes_from_mojang)?;
            }
            Ok((hash, bytes_from_mojang))
        }
    }
}
