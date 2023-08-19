use std::collections::HashMap;

use derive_more::Debug;
use strum::EnumCount;
use tracing::instrument;

use crate::error::Result;

use super::request::{
    entry::{RenderRequestEntry, RenderRequestEntryModel},
    RenderRequest, RequestRenderFeatures,
};

mod mojang;

pub(crate) struct RenderRequestResolver {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::IntoStaticStr, strum::EnumIter)]
pub enum ResolvedRenderEntryTextureType {
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
    pub(crate) textures: HashMap<ResolvedRenderEntryTextureType, MojangTexture>,
}

pub(crate) struct ResolvedRenderEntryTexturesMarker {
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
    pub(crate) fn new(
        textures: HashMap<ResolvedRenderEntryTextureType, MojangTexture>,
        model: Option<RenderRequestEntryModel>,
    ) -> Self {
        Self { textures, model }
    }

    pub(crate) fn new_from_marker_slice(
        textures: HashMap<ResolvedRenderEntryTextureType, MojangTexture>,
        marker: &[u8],
    ) -> Self {
        let model = RenderRequestEntryModel::from_repr(marker[0] as usize);

        Self { textures, model }
    }

    pub(crate) fn to_marker_slice(&self) -> [u8; 1] {
        let model = self
            .model
            .map(|m| m as u8)
            .unwrap_or(RenderRequestEntryModel::COUNT as u8);

        [model]
    }
}

impl RenderRequestResolver {
    pub(crate) fn new() -> Self {
        Self {}
    }

    async fn fetch_texture_from_mojang(&self, texture_id: &str) -> Result<MojangTexture> {
        unimplemented!()
        //if let Some(result) = self.model_cache.get_cached_texture(texture_id)? {
        //    return Ok(result);
        //}

        //let bytes = requests::fetch_texture_from_mojang(
        //    &texture_id,
        //    &self.mojang_requests_client,
        //    &self.mojank_config.textures_server,
        //)
        //.await
        //.map(|r| r.to_vec())?;

        //let _ = self.model_cache.cache_texture(&bytes, texture_id)?;

        //Ok(MojangTexture::new_named(texture_id.to_owned(), bytes))
    }

    #[instrument(skip(self))]
    async fn resolve_entry_textures(
        &self,
        entry: RenderRequestEntry,
    ) -> Result<ResolvedRenderEntryTextures> {/* 
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
                let result = requests::get_unwrapped_gameprofile(
                    &self.mojang_requests_client,
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
            textures.insert(ResolvedRenderEntryTextureType::Skin, skin_texture);
        }
        if let Some(cape_texture) = cape_texture {
            textures.insert(ResolvedRenderEntryTextureType::Cape, cape_texture);
        }

        let result = ResolvedRenderEntryTextures::new(textures, model);

        self.model_cache.cache_resolved_entity(&entry, result).await */
        unimplemented!()
    }

    pub(crate) async fn resolve(&self, request: RenderRequest) -> Result<ResolvedRenderRequest> {
        // First, we need to resolve the skin and cape textures.
        let resolved_textures = self.resolve_entry_textures(request.entry).await?;
        let final_model = request
            .model
            .or(resolved_textures.model)
            .unwrap_or_default();

        // Load the textures into memory.
        let mut textures = HashMap::new();
        for (texture_type, texture) in resolved_textures.textures {
            textures.insert(texture_type, texture.data);
        }

        let features = request.features;

        Ok(ResolvedRenderRequest {
            model: final_model,
            textures,
            features,
        })
    }
}

#[derive(Debug)]
pub(crate) struct ResolvedRenderRequest {
    pub(crate) model: RenderRequestEntryModel,
    #[debug(skip)]
    pub(crate) textures: HashMap<ResolvedRenderEntryTextureType, Vec<u8>>,
    pub(crate) features: enumset::EnumSet<RequestRenderFeatures>,
}
