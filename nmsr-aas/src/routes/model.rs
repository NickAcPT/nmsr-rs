use actix_web::web::Bytes;
use parking_lot::RwLock;
use reqwest_middleware::ClientWithMiddleware;
use tracing::trace_span;

use crate::mojang::caching::MojangCacheManager;
use crate::mojang::requests;
use crate::mojang::requests::CachedSkinHash;
use crate::utils::errors::NMSRaaSError;
use crate::utils::Result;

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
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, cache_manager, client, _span), parent = _span))]
    async fn fetch_skin_hash_and_model(
        &self,
        cache_manager: &RwLock<MojangCacheManager>,
        client: &ClientWithMiddleware,
        _span: &tracing::Span,
    ) -> Result<CachedSkinHash> {
        Ok(match self {
            PlayerRenderInput::PlayerUuid(id) => {
                let option = cache_manager
                    .read()
                    .get_cached_uuid_to_skin_hash(id)
                    .cloned();

                if let Some(hash) = option {
                    hash
                } else {
                    let limiter = {
                        let _guard_span = trace_span!("read_rate_limiter_acquire").entered();

                        let guard = cache_manager.read();
                        guard.rate_limiter.clone()
                    };
                    let result =
                        { requests::get_skin_hash_and_model(client, &limiter, *id) }.await?;

                    {
                        let _guard_span = trace_span!("write_rate_limiter_acquire").entered();
                        let mut guard = cache_manager.write();
                        drop(_guard_span);

                        guard.cache_uuid_to_skin_hash_and_model(id, result.clone());
                    }

                    result
                }
            }
            PlayerRenderInput::TextureHash(hash) => CachedSkinHash::WithoutModel {
                skin_hash: hash.clone(),
            },
        })
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(cache_manager, client, _span), parent = _span))]
    pub(crate) async fn fetch_skin_bytes(
        &self,
        cache_manager: &RwLock<MojangCacheManager>,
        client: &ClientWithMiddleware,
        _span: &tracing::Span,
    ) -> Result<(CachedSkinHash, Bytes)> {
        let current_span = tracing::Span::current();
        let cached = self
            .fetch_skin_hash_and_model(cache_manager, client, &current_span)
            .await?;

        let skin_hash = cached.get_hash();

        let result = {
            let _guard_span = trace_span!(parent: &current_span, "read_cache_acquire").entered();
            let read_guard = cache_manager.read();
            drop(_guard_span);
            read_guard.get_cached_skin(skin_hash)?
        };

        if let Some(bytes) = result {
            Ok((cached, Bytes::from(bytes)))
        } else {
            let bytes_from_mojang =
                requests::fetch_skin_bytes_from_mojang(skin_hash, client).await?;
            {
                let _guard_span = trace_span!(parent: &current_span, "write_cache_acquire").entered();
                let write_guard = cache_manager.write();
                drop(_guard_span);

                write_guard.cache_skin(skin_hash, &bytes_from_mojang)?;
            }
            Ok((cached, bytes_from_mojang))
        }
    }
}
