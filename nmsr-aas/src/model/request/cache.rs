use std::{
    borrow::Cow,
    collections::HashMap,
    fs::Metadata,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use super::entry::RenderRequestEntry;
use crate::error::{ExplainableExt, ModelCacheError, ModelCacheResult, Result};
#[cfg(feature = "ears")]
use crate::model::resolver::ResolvedRenderEntryEarsTextureType;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;
use tokio::fs;
use tracing::trace;

use crate::{
    caching::{CacheHandler, CacheSystem},
    config::ModelCacheConfiguration,
    model::resolver::{MojangTexture, ResolvedRenderEntryTextureType, ResolvedRenderEntryTextures},
};

#[serde_as]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deserialize, Serialize, strum::IntoStaticStr)]
pub enum CacheBias {
    KeepCachedFor(#[serde(with = "humantime_serde")] Duration),
    CacheIndefinitely,
}

impl TryFrom<String> for CacheBias {
    type Error = ModelCacheError;

    fn try_from(value: String) -> ModelCacheResult<Self> {
        if value == Into::<&'static str>::into(Self::CacheIndefinitely) {
            return Ok(Self::CacheIndefinitely);
        }

        let duration: Duration = humantime_serde::deserialize(Value::String(value.clone()))
            .map_err(|_| ModelCacheError::InvalidCacheBiasConfiguration(value.clone()))?;

        Ok(Self::KeepCachedFor(duration))
    }
}

impl TryFrom<CacheBias> for String {
    type Error = ModelCacheError;
    fn try_from(value: CacheBias) -> ModelCacheResult<Self> {
        Ok(match value {
            CacheBias::KeepCachedFor(duration) => {
                humantime_serde::re::humantime::format_duration(duration).to_string()
            }
            CacheBias::CacheIndefinitely => {
                Into::<&'static str>::into(CacheBias::CacheIndefinitely).to_string()
            }
        })
    }
}

struct MojangTextureCacheHandler;

struct ResolvedModelTexturesCacheHandler {
    mojang_texture_cache: Arc<
        CacheSystem<str, MojangTexture, ModelCacheConfiguration, (), MojangTextureCacheHandler>,
    >,
}

#[async_trait]
#[allow(unused_variables)]
impl CacheHandler<str, MojangTexture, ModelCacheConfiguration, ()> for MojangTextureCacheHandler {
    #[inline]
    async fn get_cache_key(
        &self,
        entry: &str,
        _config: &ModelCacheConfiguration,
    ) -> Result<Option<String>> {
        Ok(Some(entry.to_string()))
    }

    #[inline]
    async fn read_key_from_path<'a>(
        &'a self,
        _config: &ModelCacheConfiguration,
        path: &'a Path,
    ) -> Result<Option<Cow<'a, str>>> {
        Ok(path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .map(std::convert::Into::into))
    }

    async fn get_marker_path(
        &self,
        entry: &str,
        config: &ModelCacheConfiguration,
    ) -> Result<String> {
        Ok(String::new())
    }

    fn is_expired(
        &self,
        entry: &str,
        config: &ModelCacheConfiguration,
        _marker: &(),
        marker_metadata: Metadata,
    ) -> Result<bool> {
        config.is_expired_with_default(
            &RenderRequestEntry::TextureHash(entry.to_string()),
            &marker_metadata,
            &config.texture_cache_duration,
        )
    }

    async fn write_cache(
        &self,
        entry: &str,
        value: &MojangTexture,
        _config: &ModelCacheConfiguration,
        file: &Path,
    ) -> Result<()> {
        fs::write(file, value.data())
            .await
            .explain(format!("Unable to write texture {entry:?} to cache"))?;

        Ok(())
    }

    async fn read_cache(
        &self,
        entry: &str,
        config: &ModelCacheConfiguration,
        file: &Path,
        _marker: &(),
    ) -> Result<Option<MojangTexture>> {
        if !file.exists() {
            return Ok(None);
        }

        let data = fs::read(file)
            .await
            .explain(format!("Unable to read texture {entry:?} from cache"))?;

        if !config.validate_png_data(&data) {
            trace!("Texture {entry:?} is invalid, discarding.");
            CacheSystem::<str, MojangTexture, ModelCacheConfiguration, (), Self>::invalidate_self(
                entry, file,
            )
            .await?;
            return Ok(None);
        }

        Ok(Some(MojangTexture::new_named(entry.to_string(), data)))
    }

    async fn read_marker(
        &self,
        _entry: &str,
        _config: &ModelCacheConfiguration,
        _marker: &Path,
    ) -> Result<()> {
        Ok(())
    }

    async fn write_marker(
        &self,
        _entry: &str,
        _value: &MojangTexture,
        _config: &ModelCacheConfiguration,
        _marker: &Path,
    ) -> Result<()> {
        Ok(())
    }
}

pub struct ModelCache {
    mojang: Arc<
        CacheSystem<str, MojangTexture, ModelCacheConfiguration, (), MojangTextureCacheHandler>,
    >,
    resolved_textures: CacheSystem<
        RenderRequestEntry,
        ResolvedRenderEntryTextures,
        ModelCacheConfiguration,
        [u8; 1],
        ResolvedModelTexturesCacheHandler,
    >,
}

impl ModelCache {
    pub async fn new(cache_path: PathBuf, cache_config: ModelCacheConfiguration) -> Result<Self> {
        let mojang = CacheSystem::new(
            cache_path.join("textures"),
            cache_config.clone(),
            MojangTextureCacheHandler,
        )
        .await?;

        let mojang = Arc::new(mojang);

        let resolved = CacheSystem::new(
            cache_path.join("resolved"),
            cache_config.clone(),
            ResolvedModelTexturesCacheHandler {
                mojang_texture_cache: mojang.clone(),
            },
        )
        .await?;

        Ok(Self {
            mojang: mojang.clone(),
            resolved_textures: resolved,
        })
    }

    pub async fn get_cached_texture(&self, texture_id: &str) -> Result<Option<MojangTexture>> {
        self.mojang.get_cached_entry(texture_id).await
    }

    pub async fn cache_texture(&self, texture: &MojangTexture) -> Result<()> {
        if let Some(hash) = texture.hash() {
            self.mojang.set_cache_entry(hash, texture).await.map(|_| ())
        } else {
            Ok(())
        }
    }

    pub async fn get_cached_resolved_texture(
        &self,
        entry: &RenderRequestEntry,
    ) -> Result<Option<ResolvedRenderEntryTextures>> {
        self.resolved_textures.get_cached_entry(entry).await
    }

    pub async fn cache_resolved_texture(
        &self,
        entry: &RenderRequestEntry,
        textures: &ResolvedRenderEntryTextures,
    ) -> Result<()> {
        self.resolved_textures
            .set_cache_entry(entry, textures)
            .await
            .map(|_| ())
    }

    pub(crate) async fn do_cache_clean_up(&self) -> Result<()> {
        self.resolved_textures.perform_cache_cleanup().await?;
        self.mojang.perform_cache_cleanup().await?;

        Ok(())
    }
}

#[async_trait]
impl CacheHandler<RenderRequestEntry, ResolvedRenderEntryTextures, ModelCacheConfiguration, [u8; 1]>
    for ResolvedModelTexturesCacheHandler
{
    #[inline]
    async fn get_cache_key(
        &self,
        entry: &RenderRequestEntry,
        _config: &ModelCacheConfiguration,
    ) -> Result<Option<String>> {
        Ok(match entry {
            RenderRequestEntry::MojangPlayerUuid(u)
            | RenderRequestEntry::MojangOfflinePlayerUuid(u)
            | RenderRequestEntry::GeyserPlayerUuid(u) => Some(u.to_string()),
            RenderRequestEntry::TextureHash(hash) => Some(hash.clone()),
            RenderRequestEntry::PlayerSkin(_, _) => None,
        })
    }

    #[inline]
    async fn read_key_from_path<'a>(
        &'a self,
        _config: &ModelCacheConfiguration,
        path: &'a Path,
    ) -> Result<Option<Cow<'a, RenderRequestEntry>>> {
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .to_string();

        let entry = RenderRequestEntry::try_from(file_name)?;

        Ok(Some(Cow::Owned(entry)))
    }

    fn is_expired(
        &self,
        entry: &RenderRequestEntry,
        config: &ModelCacheConfiguration,
        _marker: &[u8; 1],
        marker_metadata: Metadata,
    ) -> Result<bool> {
        config.is_expired(entry, &marker_metadata)
    }

    async fn write_cache(
        &self,
        entry: &RenderRequestEntry,
        value: &ResolvedRenderEntryTextures,
        _config: &ModelCacheConfiguration,
        base: &Path,
    ) -> Result<()> {
        if !base.exists() {
            fs::create_dir_all(base)
                .await
                .explain(format!("Unable to create cache directory for {:?}", &entry))?;
        }

        for (texture_type, texture) in &value.textures {
            let texture_path =
                base.join(format!("{}{}", Into::<&str>::into(*texture_type), ".png"));

            if let Some(texture_hash) = texture.hash() {
                let cache_path = self
                    .mojang_texture_cache
                    .set_cache_entry(texture_hash.as_str(), texture)
                    .await?;

                if let Some(cache_path) = cache_path {
                    let cache_path = cache_path.canonicalize().explain(format!(
                        "Unable to canonicalize cache path for texture {texture_hash:?} for {entry:?}"
                    ))?;

                    symlink::symlink_file(cache_path, texture_path).explain(format!(
                        "Unable to create symlink for texture {texture_hash:?} for {entry:?}"
                    ))?;
                }
            }
        }

        Ok(())
    }

    async fn read_cache(
        &self,
        entry: &RenderRequestEntry,
        config: &ModelCacheConfiguration,
        base: &Path,
        marker: &[u8; 1],
    ) -> Result<Option<ResolvedRenderEntryTextures>> {
        let mut textures = HashMap::new();

        let textures_to_read = [
            ResolvedRenderEntryTextureType::Skin,
            ResolvedRenderEntryTextureType::Cape,
            #[cfg(feature = "ears")]
            ResolvedRenderEntryTextureType::Ears(ResolvedRenderEntryEarsTextureType::Wings),
            #[cfg(feature = "ears")]
            ResolvedRenderEntryTextureType::Ears(ResolvedRenderEntryEarsTextureType::Cape),
            #[cfg(feature = "ears")]
            ResolvedRenderEntryTextureType::Ears(ResolvedRenderEntryEarsTextureType::Emissive),
        ];

        for texture in textures_to_read {
            let is_important_texture = matches!(texture, ResolvedRenderEntryTextureType::Skin | ResolvedRenderEntryTextureType::Cape);

            let texture_path = base.join(format!("{}{}", Into::<&str>::into(texture), ".png"));

            if texture_path.exists() {
                let read = fs::read(texture_path).await.explain(format!(
                    "Unable to read texture {:?} for {:?}",
                    &texture, &entry
                ))?;

                if is_important_texture && !config.validate_png_data(&read) {
                    trace!("Texture {texture:?} for {entry:?} is invalid, discarding.");
                    CacheSystem::<
                        RenderRequestEntry,
                        ResolvedRenderEntryTextures,
                        ModelCacheConfiguration,
                        [u8; 1],
                        Self,
                    >::invalidate_self(entry, base)
                    .await?;
                
                    return Ok(None);
                }

                textures.insert(texture, MojangTexture::new_unnamed(read));
            } else if !is_important_texture {
                // If we haven't found a cached texture for an important texture, then we just skip
                continue;
            } else {
                trace!(
                    "Unable to find texture path for important texture {}",
                    texture_path.display()
                );

                // One of the textures has gone missing, this means that this cache entry is invalid and should be removed
                CacheSystem::<
                    RenderRequestEntry,
                    ResolvedRenderEntryTextures,
                    ModelCacheConfiguration,
                    [u8; 1],
                    Self,
                >::invalidate_self(entry, base)
                .await?;
                return Ok(None);
            }
        }

        Ok(Some(ResolvedRenderEntryTextures::new_from_marker_slice(
            textures, marker,
        )))
    }

    async fn read_marker(
        &self,
        entry: &RenderRequestEntry,
        _config: &ModelCacheConfiguration,
        marker: &Path,
    ) -> Result<[u8; 1]> {
        let result = fs::read(marker)
            .await
            .explain(format!("Unable to read marker file for {entry:?}"))?;

        if result.len() != 1 {
            return Err(ModelCacheError::MarkerMetadataError(entry.clone()).into());
        }

        Ok([result[0]])
    }

    async fn write_marker(
        &self,
        entry: &RenderRequestEntry,
        value: &ResolvedRenderEntryTextures,
        _config: &ModelCacheConfiguration,
        marker: &Path,
    ) -> Result<()> {
        fs::write(marker, value.to_marker_slice())
            .await
            .explain(format!("Unable to write marker file for {entry:?}"))?;

        Ok(())
    }
}
