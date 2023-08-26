use std::{collections::HashMap, sync::Arc};

use derive_more::Debug;
use nmsr_rendering::high_level::types::PlayerPartTextureType;
use strum::EnumCount;
use tracing::{instrument, Span};

use crate::{
    config::NmsrConfiguration,
    error::{MojangRequestError, NMSRaaSError, Result},
};

use self::mojang::{client::MojangClient, model::GameProfileTexture};

use super::request::{
    cache::ModelCache,
    entry::{RenderRequestEntry, RenderRequestEntryModel},
    RenderRequest,
};

pub mod mojang;

pub struct RenderRequestResolver {
    model_cache: ModelCache,
    mojang_requests_client: Arc<MojangClient>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::IntoStaticStr, strum::EnumIter)]
pub enum ResolvedRenderEntryTextureType {
    Skin,
    Cape,
    #[cfg(feature = "ears")]
    Ears,
}

impl From<ResolvedRenderEntryTextureType> for PlayerPartTextureType {
    fn from(value: ResolvedRenderEntryTextureType) -> Self {
        match value {
            ResolvedRenderEntryTextureType::Skin => PlayerPartTextureType::Skin,
            ResolvedRenderEntryTextureType::Cape => PlayerPartTextureType::Cape,
            #[cfg(feature = "ears")]
            ResolvedRenderEntryTextureType::Ears => PlayerPartTextureType::Ears,
        }
    }
}

pub struct MojangTexture {
    hash: Option<String>,
    data: Vec<u8>,
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

    pub fn hash(&self) -> Option<&String> {
        self.hash.as_ref()
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_ref()
    }
}

pub struct ResolvedRenderEntryTextures {
    pub model: Option<RenderRequestEntryModel>,
    pub textures: HashMap<ResolvedRenderEntryTextureType, MojangTexture>,
}

pub struct ResolvedRenderEntryTexturesMarker {
    pub model: u8,
}

impl From<ResolvedRenderEntryTextures> for ResolvedRenderEntryTexturesMarker {
    fn from(value: ResolvedRenderEntryTextures) -> Self {
        let model = if let Some(value) = value.model {
            value as u8
        } else {
            RenderRequestEntryModel::COUNT as u8
        };

        ResolvedRenderEntryTexturesMarker { model }
    }
}

impl ResolvedRenderEntryTextures {
    pub fn new(
        textures: HashMap<ResolvedRenderEntryTextureType, MojangTexture>,
        model: Option<RenderRequestEntryModel>,
    ) -> Self {
        Self { textures, model }
    }

    pub fn new_from_marker_slice(
        textures: HashMap<ResolvedRenderEntryTextureType, MojangTexture>,
        marker: &[u8],
    ) -> Self {
        let model = RenderRequestEntryModel::from_repr(marker[0] as usize);

        Self { textures, model }
    }

    pub fn to_marker_slice(&self) -> [u8; 1] {
        let model = self
            .model
            .map(|m| m as u8)
            .unwrap_or(RenderRequestEntryModel::COUNT as u8);

        [model]
    }
}

impl RenderRequestResolver {
    pub fn new(model_cache: ModelCache, client: Arc<MojangClient>) -> Self {
        Self {
            model_cache,
            mojang_requests_client: client,
        }
    }

    async fn fetch_game_profile_texture(
        &self,
        texture: Option<&GameProfileTexture>,
    ) -> Result<Option<MojangTexture>> {
        if let Some(texture) = texture {
            let texture_id = texture.hash()?;

            let texture = self.fetch_texture_from_mojang(texture_id).await?;

            Ok(Some(texture))
        } else {
            Ok(None)
        }
    }

    async fn fetch_texture_from_mojang(&self, texture_id: &str) -> Result<MojangTexture> {
        if let Some(result) = self.model_cache.get_cached_texture(texture_id).await? {
            return Ok(result);
        }

        let bytes = self
            .mojang_requests_client
            .fetch_texture_from_mojang(&texture_id, &Span::current())
            .await?;

        let texture = MojangTexture::new_named(texture_id.to_owned(), bytes);

        self.model_cache.cache_texture(&texture).await?;

        Ok(texture)
    }

    #[instrument(skip(self))]
    async fn resolve_entry_textures(
        &self,
        entry: &RenderRequestEntry,
    ) -> Result<ResolvedRenderEntryTextures> {
        if let Some(result) = self.model_cache.get_cached_resolved_texture(&entry).await? {
            return Ok(result);
        }

        let model: Option<RenderRequestEntryModel>;
        let skin_texture: Option<MojangTexture>;
        let cape_texture: Option<MojangTexture>;
        #[cfg(feature = "ears")]
        let mut ears_texture = compile_error!("Implement ears texture");

        match &entry {
            RenderRequestEntry::PlayerUuid(id) => {
                let result = self
                    .mojang_requests_client
                    .resolve_uuid_to_game_profile(id)
                    .await?;
                let textures = result.textures()?;

                let skin = textures
                    .skin()
                    .ok_or_else(|| MojangRequestError::MissingSkinPropertyError(id.clone()))?;
                let cape = textures.cape();

                model = if skin.is_slim() {
                    Some(RenderRequestEntryModel::Alex)
                } else {
                    Some(RenderRequestEntryModel::Steve)
                };

                skin_texture = self.fetch_game_profile_texture(textures.skin()).await?;
                cape_texture = self.fetch_game_profile_texture(cape).await?;
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
            textures.insert(ResolvedRenderEntryTextureType::Skin, skin_texture);
        }
        if let Some(cape_texture) = cape_texture {
            textures.insert(ResolvedRenderEntryTextureType::Cape, cape_texture);
        }

        let result = ResolvedRenderEntryTextures::new(textures, model);

        self.model_cache
            .cache_resolved_texture(&entry, &result)
            .await?;

        Ok(result)
    }

    pub async fn resolve(&self, request: &RenderRequest) -> Result<ResolvedRenderRequest> {
        // First, we need to resolve the skin and cape textures.
        let resolved_textures = self
            .resolve_entry_textures(&request.entry)
            .await
            .map_err(|e| {
                MojangRequestError::UnableToResolveRenderRequestEntity(
                    Box::new(e),
                    request.entry.clone(),
                )
            })?;

        let final_model = request
            .model
            .or(resolved_textures.model)
            .unwrap_or_default();

        // Load the textures into memory.
        let mut textures = HashMap::new();
        for (texture_type, texture) in resolved_textures.textures {
            textures.insert(texture_type, texture.data);
        }

        Ok(ResolvedRenderRequest {
            model: final_model,
            textures,
        })
    }
}

#[derive(Debug)]
pub struct ResolvedRenderRequest {
    pub model: RenderRequestEntryModel,
    #[debug(skip)]
    pub textures: HashMap<ResolvedRenderEntryTextureType, Vec<u8>>,
}
