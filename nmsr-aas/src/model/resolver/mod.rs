use self::{
    geyser::resolve_geyser_uuid_to_texture_and_model,
    mojang::{client::MojangClient, model::GameProfileTexture},
};
use super::request::{
    cache::ModelCache,
    entry::{RenderRequestEntry, RenderRequestEntryModel},
    RenderRequest,
};
use crate::{error::{MojangRequestError, RenderRequestError, Result}, model::resolver::mojang::client::MojangTextureRequestType};
use derive_more::Debug;
#[cfg(feature = "ears")]
use ears_rs::{alfalfa::AlfalfaDataKey, features::EarsFeatures, parser::EarsParser};
#[cfg(feature = "ears")]
use nmsr_rendering::high_level::parts::provider::ears::PlayerPartEarsTextureType;
use nmsr_rendering::high_level::types::PlayerPartTextureType;
use std::{collections::HashMap, sync::Arc};
use strum::EnumCount;
use tracing::{instrument, trace_span, Instrument, Span};

pub mod geyser;
pub mod mojang;

pub struct RenderRequestResolver {
    model_cache: ModelCache,
    mojang_requests_client: Arc<MojangClient>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolvedRenderEntryTextureType {
    Cape,
    Skin,
    #[cfg(feature = "ears")]
    Ears(ResolvedRenderEntryEarsTextureType),
}

impl From<ResolvedRenderEntryTextureType> for &'static str {
    fn from(value: ResolvedRenderEntryTextureType) -> Self {
        match value {
            ResolvedRenderEntryTextureType::Cape => "Cape",
            ResolvedRenderEntryTextureType::Skin => "Skin",
            #[cfg(feature = "ears")]
            ResolvedRenderEntryTextureType::Ears(ears) => ears.key(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolvedRenderEntryEarsTextureType {
    Cape,
    Wings,
    Emissive,
}

#[cfg(feature = "ears")]
impl From<ResolvedRenderEntryEarsTextureType> for PlayerPartEarsTextureType {
    fn from(value: ResolvedRenderEntryEarsTextureType) -> Self {
        match value {
            ResolvedRenderEntryEarsTextureType::Cape => Self::Cape,
            ResolvedRenderEntryEarsTextureType::Wings => Self::Wings,
            ResolvedRenderEntryEarsTextureType::Emissive => Self::Emissive,
        }
    }
}

#[cfg(feature = "ears")]
impl ResolvedRenderEntryEarsTextureType {
    const fn alfalfa_key(self) -> Option<AlfalfaDataKey> {
        match self {
            Self::Cape => Some(AlfalfaDataKey::Cape),
            Self::Wings => Some(AlfalfaDataKey::Wings),
            Self::Emissive => None,
        }
    }

    fn key(self) -> &'static str {
        PlayerPartEarsTextureType::from(self).key()
    }
}

impl From<ResolvedRenderEntryTextureType> for PlayerPartTextureType {
    fn from(value: ResolvedRenderEntryTextureType) -> Self {
        match value {
            ResolvedRenderEntryTextureType::Skin => Self::Skin,
            ResolvedRenderEntryTextureType::Cape => Self::Cape,
            #[cfg(feature = "ears")]
            ResolvedRenderEntryTextureType::Ears(ResolvedRenderEntryEarsTextureType::Cape) => {
                Self::Cape
            }
            #[cfg(feature = "ears")]
            ResolvedRenderEntryTextureType::Ears(ears) => {
                PlayerPartEarsTextureType::from(ears).into()
            }
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

    #[must_use]
    pub const fn hash(&self) -> Option<&String> {
        self.hash.as_ref()
    }

    #[must_use]
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
        let model = value
            .model
            .map_or(RenderRequestEntryModel::COUNT as u8, |value| value as u8);

        Self { model }
    }
}

impl ResolvedRenderEntryTextures {
    #[must_use]
    pub const fn new(
        textures: HashMap<ResolvedRenderEntryTextureType, MojangTexture>,
        model: Option<RenderRequestEntryModel>,
    ) -> Self {
        Self { model, textures }
    }

    #[must_use]
    pub fn new_from_marker_slice(
        textures: HashMap<ResolvedRenderEntryTextureType, MojangTexture>,
        marker: &[u8],
    ) -> Self {
        let model = RenderRequestEntryModel::from_repr(marker[0] as usize);

        Self { model, textures }
    }

    #[must_use]
    pub fn to_marker_slice(&self) -> [u8; 1] {
        let model = self
            .model
            .map_or(RenderRequestEntryModel::COUNT as u8, |m| m as u8);

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
        req_type: MojangTextureRequestType,
    ) -> Result<Option<MojangTexture>> {
        if let Some(texture) = texture {
            let texture_id = texture.hash()?;

            let texture = self.fetch_texture_from_mojang(texture_id, req_type).await?;

            Ok(Some(texture))
        } else {
            Ok(None)
        }
    }

    #[instrument(skip(self), parent = &Span::current())]
    async fn fetch_texture_from_mojang(&self, texture_id: &str, req_type: MojangTextureRequestType) -> Result<MojangTexture> {
        if let Some(result) = self.model_cache.get_cached_texture(texture_id).await? {
            return Ok(result);
        }

        let bytes = self
            .mojang_requests_client
            .fetch_texture_from_mojang(texture_id, req_type)
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
        if let Some(result) = self.model_cache.get_cached_resolved_texture(entry).await? {
            return Ok(result);
        }

        let model: Option<RenderRequestEntryModel>;
        let skin_texture: Option<MojangTexture>;
        let cape_texture: Option<MojangTexture>;

        match &entry {
            RenderRequestEntry::MojangPlayerUuid(id) | RenderRequestEntry::MojangOfflinePlayerUuid(id) => {
                if matches!(&entry, RenderRequestEntry::MojangOfflinePlayerUuid(_)) && !self.mojang_requests_client.mojank_config().allow_offline_mode_uuids {
                    return Err(RenderRequestError::InvalidPlayerUuidRequest(
                        id.to_string(),
                        id.get_version_num(),
                    ))?;                    
                }
                
                let result = self
                    .mojang_requests_client
                    .resolve_uuid_to_game_profile(id)
                    .instrument(trace_span!("resolve_uuid_to_game_profile", uuid = %id))
                    .await?;
                
                let textures = result.textures()?;

                let skin = textures
                    .skin()
                    .ok_or_else(|| MojangRequestError::MissingSkinPropertyError(*id))?;
                let cape = textures.cape();

                model = if skin.is_slim() {
                    Some(RenderRequestEntryModel::Alex)
                } else {
                    Some(RenderRequestEntryModel::Steve)
                };

                skin_texture = self.fetch_game_profile_texture(textures.skin(), MojangTextureRequestType::Skin).await?;
                cape_texture = self.fetch_game_profile_texture(cape, MojangTextureRequestType::Cape).await?;
            }
            RenderRequestEntry::GeyserPlayerUuid(id) => {
                let (texture_id, player_model) =
                    resolve_geyser_uuid_to_texture_and_model(&self.mojang_requests_client, id)
                        .await?;

                skin_texture = Some(self.fetch_texture_from_mojang(&texture_id, MojangTextureRequestType::Skin).await?);
                cape_texture = None;

                model = Some(player_model);
            }
            RenderRequestEntry::TextureHash(skin_hash) => {
                // If the skin is not cached, we'll have to fetch it from Mojang.
                skin_texture = Some(self.fetch_texture_from_mojang(skin_hash, MojangTextureRequestType::Skin).await?);
                cape_texture = None;
                model = None;
            }
            RenderRequestEntry::PlayerSkin(skin_bytes, cape_bytes) => {
                skin_texture = Some(MojangTexture::new_unnamed(skin_bytes.clone()));
                cape_texture = cape_bytes.to_owned().map(|b| MojangTexture::new_unnamed(b));
                model = None;
            }
        }

        let mut textures = HashMap::new();

        if let Some(cape_texture) = cape_texture {
            textures.insert(ResolvedRenderEntryTextureType::Cape, cape_texture);
        }

        if let Some(skin_texture) = skin_texture {
            #[cfg(feature = "ears")]
            Self::resolve_ears_textures(&skin_texture, &mut textures);

            textures.insert(ResolvedRenderEntryTextureType::Skin, skin_texture);
        }

        let result = ResolvedRenderEntryTextures::new(textures, model);

        self.model_cache
            .cache_resolved_texture(entry, &result)
            .await?;

        Ok(result)
    }

    #[cfg(feature = "ears")]
    fn resolve_ears_textures(
        skin_texture: &MojangTexture,
        textures: &mut HashMap<ResolvedRenderEntryTextureType, MojangTexture>,
    ) -> Option<EarsFeatures> {
        use std::borrow::Cow;
        use image::DynamicImage;
        use xxhash_rust::xxh3::xxh3_128;
        use crate::utils::png::create_png_from_bytes;

        image::load_from_memory(skin_texture.data()).map_or(None, |image| {
            let image = image.into_rgba8();

            let features = EarsParser::parse(&image).ok().flatten();
            let alfalfa = ears_rs::alfalfa::read_alfalfa(&image).ok().flatten();

            if let Some(alfalfa) = alfalfa {
                for texture_type in &[
                    ResolvedRenderEntryEarsTextureType::Cape,
                    ResolvedRenderEntryEarsTextureType::Wings,
                ] {
                    if let Some(alfalfa_key) = texture_type.alfalfa_key() {
                        if let Some(data) = alfalfa.get_data(alfalfa_key) {
                            let hash = format!("{:x}", xxh3_128(data));

                            let data = if alfalfa_key == AlfalfaDataKey::Cape {
                                let image = image::load_from_memory(data)
                                    .map(DynamicImage::into_rgba8)
                                    .map(ears_rs::utils::convert_ears_cape_to_mojang_cape)
                                    .ok()
                                    .and_then(|i| {
                                        create_png_from_bytes((i.width(), i.height()), &i).ok()
                                    });

                                image.map_or(Cow::Borrowed(data), Cow::Owned)
                            } else {
                                Cow::Borrowed(data)
                            };

                            textures.insert(
                                ResolvedRenderEntryTextureType::Ears(*texture_type),
                                MojangTexture::new_named(hash, data.into_owned()),
                            );
                        }
                    }
                }
            }

            features
        })
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

    #[inline]
    pub(crate) async fn do_cache_clean_up(&self) -> Result<()> {
        self.model_cache.do_cache_clean_up().await
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedRenderRequest {
    pub model: RenderRequestEntryModel,
    #[debug(skip)]
    pub textures: HashMap<ResolvedRenderEntryTextureType, Vec<u8>>,
}
