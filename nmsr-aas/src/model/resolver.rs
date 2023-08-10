use crate::config::{CacheConfiguration, MojankConfiguration};
use crate::model::RenderRequest;
use crate::mojang::caching::MojangCacheManager;
use crate::mojang::requests;
use crate::utils::Result;
use parking_lot::RwLock;
use reqwest_middleware::ClientWithMiddleware;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{trace_span, Span};

use super::RenderRequestEntry;

struct RenderRequestResolver {
    cache_config: Arc<CacheConfiguration>,
    mojang_requests_client: Arc<ClientWithMiddleware>,
    cache_manager: Arc<RwLock<MojangCacheManager>>,
    mojank_config: Arc<MojankConfiguration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RenderEntryTextureType {
    Skin,
    Cape,
    #[cfg(feature = "ears")]
    Ears,
}

struct ResolvedRenderEntryTextures {
    textures: HashMap<RenderEntryTextureType, Vec<u8>>,
}

impl RenderRequestResolver {
    #[tracing::instrument(skip(self))]
    async fn resolve_entry_textures(
        &self,
        entry: RenderRequestEntry,
    ) -> Result<ResolvedRenderEntryTextures> {
        let current_span = Span::current();
        let skin_texture: Option<Vec<u8>>;
        let cape_texture: Option<Vec<u8>> = None;
        #[cfg(feature = "ears")]
        let mut ears_texture = todo!("Implement ears texture");

        match entry {
            RenderRequestEntry::PlayerUuid(_id) => {
                todo!("Request profile from Mojang");
            }
            RenderRequestEntry::TextureHash(skin_hash) => {
                // First, we need to check if the skin is cached.
                let result = { self.cache_manager.read().get_cached_skin(&skin_hash)? };

                if let Some(bytes) = result {
                    // If the skin is cached, we'll use that.
                    skin_texture = Some(bytes);
                } else {
                    // If the skin is not cached, we'll have to fetch it from Mojang.
                    let bytes_from_mojang = requests::fetch_skin_bytes_from_mojang(
                        &skin_hash,
                        &self.mojang_requests_client,
                        &self.mojank_config.textures_server,
                    )
                    .await?;

                    // We'll also cache the skin for future use.
                    skin_texture = Some(bytes_from_mojang.to_vec());

                    // Cache the skin for future use.
                    // TODO: Move this logic to all the other texture types.
                    {
                        let _guard_span =
                            trace_span!(parent: &current_span, "write_cache_acquire").entered();
                        let write_guard = self.cache_manager.write();
                        drop(_guard_span);

                        write_guard.cache_skin(&skin_hash, &bytes_from_mojang)?;
                    }
                }
            }
            RenderRequestEntry::PlayerSkin(bytes) => {
                skin_texture = Some(bytes);
            }
        }

        let mut textures = HashMap::new();

        if let Some(skin_texture) = skin_texture {
            textures.insert(RenderEntryTextureType::Skin, skin_texture);
        }
        if let Some(cape_texture) = cape_texture {
            textures.insert(RenderEntryTextureType::Cape, cape_texture);
        }

        Ok(ResolvedRenderEntryTextures { textures })
    }

    async fn resolve(&self, request: RenderRequest) -> Result<()> {
        // First, we need to resolve the skin and cape textures.
        let _resolved_textures = self.resolve_entry_textures(request.entry).await?;
        
        unimplemented!()
    }
}
