use crate::config::{CacheConfiguration, MojankConfiguration};
use crate::model::RenderRequest;
use crate::mojang::caching::MojangCacheManager;
use crate::mojang::requests;
use crate::utils::Result;
use image::RgbaImage;
use parking_lot::RwLock;
use reqwest_middleware::ClientWithMiddleware;
use strum::EnumCount;
use std::collections::HashMap;
use std::sync::Arc;

use super::caching_v2::ModelCache;
use super::{RenderRequestEntry, RenderRequestEntryModel};

#[cfg(feature = "tracing")]
use tracing::instrument;

pub(crate) struct RenderRequestResolver {
    cache_config: Arc<CacheConfiguration>,
    mojang_requests_client: Arc<ClientWithMiddleware>,
    cache_manager: Arc<RwLock<MojangCacheManager>>,
    mojank_config: Arc<MojankConfiguration>,
    model_cache: Arc<ModelCache>,
}

impl RenderRequestResolver {
    pub fn cache_config(&self) -> &CacheConfiguration {
        &self.cache_config
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::IntoStaticStr, strum::EnumIter)]
pub(crate) enum RenderEntryTextureType {
    Skin,
    Cape,
    #[cfg(feature = "ears")]
    Ears,
}

pub(crate) struct MojangTexture {
    pub(crate) hash: Option<String>,
    pub(crate) data: Vec<u8>,
}

impl MojangTexture {
    pub(crate) fn new_named(hash: String, data: Vec<u8>) -> Self {
        Self {
            hash: Some(hash),
            data,
        }
    }
    pub(crate) fn new_unnamed(data: Vec<u8>) -> Self {
        Self { hash: None, data }
    }
}

pub(crate) struct ResolvedRenderEntryTextures {
    pub(crate) model: Option<RenderRequestEntryModel>,
    pub(crate) textures: HashMap<RenderEntryTextureType, MojangTexture>,
}

impl ResolvedRenderEntryTextures {
    pub(crate) fn new(textures: HashMap<RenderEntryTextureType, MojangTexture>, model: Option<RenderRequestEntryModel>) -> Self {
        Self {
            textures,
            model
        }
    }
    
    pub(crate) fn new_from_marker_slice(textures: HashMap<RenderEntryTextureType, MojangTexture>, marker: &[u8]) -> Self {
        let model = RenderRequestEntryModel::from_repr(marker[0] as usize);
        
        Self {
            textures,
            model
        }        
    }
    
    pub(crate) fn to_marker_slice(&self) -> [u8; 1] {
        let model = self.model.map(|m| m as u8).unwrap_or(RenderRequestEntryModel::COUNT as u8);
        
        [model]
    }
}

impl RenderRequestResolver {
    pub(crate) fn new(
        cache_config: Arc<CacheConfiguration>,
        mojang_requests_client: Arc<ClientWithMiddleware>,
        cache_manager: Arc<RwLock<MojangCacheManager>>,
        mojank_config: Arc<MojankConfiguration>,
        model_cache: Arc<ModelCache>,
    ) -> Self {
        Self {
            cache_config,
            mojang_requests_client,
            cache_manager,
            mojank_config,
            model_cache,
        }
    }

    async fn fetch_texture_from_mojang(&self, texture_id: &str) -> Result<MojangTexture> {
        if let Some(result) = self.model_cache.get_cached_texture(texture_id)? {
            return Ok(result);
        }

        let bytes = requests::fetch_texture_from_mojang(
            &texture_id,
            &self.mojang_requests_client,
            &self.mojank_config.textures_server,
        )
        .await
        .map(|r| r.to_vec())?;

        let _ = self.model_cache.cache_texture(&bytes, texture_id)?;

        Ok(MojangTexture::new_named(texture_id.to_owned(), bytes))
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self)))]
    async fn resolve_entry_textures(
        &self,
        entry: RenderRequestEntry,
    ) -> Result<ResolvedRenderEntryTextures> {
        if let Some(result) = self.model_cache.get_cached_resolved_entity(&entry)? {
            return Ok(result);
        }

        let model: Option<RenderRequestEntryModel>;
        let skin_texture: Option<MojangTexture>;
        let cape_texture: Option<MojangTexture>;
        #[cfg(feature = "ears")]
        let mut ears_texture = todo!("Implement ears texture");

        match &entry {
            RenderRequestEntry::PlayerUuid(id) => {
                let limiter = {
                    let guard = self.cache_manager.read();
                    guard.rate_limiter.clone()
                };

                let result = requests::get_unwrapped_gameprofile(
                    &self.mojang_requests_client,
                    &limiter,
                    *id,
                    &self.mojank_config.session_server,
                )
                .await?;
            
                model = if result.slim_arms {
                    Some(RenderRequestEntryModel::Alex)
                } else {
                    Some(RenderRequestEntryModel::Steve)
                };

                skin_texture = Some(
                    self.fetch_texture_from_mojang(&result.skin_texture_hash)
                        .await?,
                );

                if let Some(cape_texture_hash) = result.cape_texture_hash {
                    cape_texture = Some(self.fetch_texture_from_mojang(&cape_texture_hash).await?);
                } else {
                    cape_texture = None;
                }
            }
            RenderRequestEntry::TextureHash(skin_hash) => {
                // If the skin is not cached, we'll have to fetch it from Mojang.
                skin_texture = Some(self.fetch_texture_from_mojang(&skin_hash).await?);
                cape_texture = None;
                model = None;
            }
            RenderRequestEntry::PlayerSkin(bytes) => {
                skin_texture = Some(MojangTexture::new_unnamed(bytes.clone()));
                cape_texture = None;
                model = None;
            }
        }

        let mut textures = HashMap::new();

        if let Some(skin_texture) = skin_texture {
            textures.insert(RenderEntryTextureType::Skin, skin_texture);
        }
        if let Some(cape_texture) = cape_texture {
            textures.insert(RenderEntryTextureType::Cape, cape_texture);
        }

        let result = ResolvedRenderEntryTextures::new(textures, model);

        self.model_cache.cache_resolved_entity(&entry, result).await
    }

    pub(crate) async fn resolve(&self, request: RenderRequest) -> Result<ResolvedRenderRequest> {
        // First, we need to resolve the skin and cape textures.
        let resolved_textures = self.resolve_entry_textures(request.entry).await?;
        let final_model = request.model.or(resolved_textures.model).unwrap_or_default();

        // Load the textures into memory.
        let mut textures = HashMap::new();
        for (texture_type, texture) in resolved_textures.textures {
            let image: RgbaImage = image::load_from_memory(&texture.data)?.into_rgba8();

            textures.insert(texture_type, image);
        }

        let features = request.features;

        Ok(ResolvedRenderRequest {
            model: final_model,
            textures,
            features,
        })
    }
}

pub(crate) struct ResolvedRenderRequest {
    pub(crate) model: RenderRequestEntryModel,
    pub(crate) textures: HashMap<RenderEntryTextureType, RgbaImage>,
    pub(crate) features: enumset::EnumSet<super::RequestRenderFeatures>,
}

impl std::fmt::Debug for ResolvedRenderRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedRenderRequest")
            .field("model", &self.model)
            .field("features", &self.features)
            .finish()
    }
}
