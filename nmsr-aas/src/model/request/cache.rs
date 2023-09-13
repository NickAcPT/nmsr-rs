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
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;
use strum::IntoEnumIterator;
use tokio::fs;

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
        if &value == Into::<&'static str>::into(CacheBias::CacheIndefinitely) {
            return Ok(CacheBias::CacheIndefinitely);
        }

        let duration: Duration = humantime_serde::deserialize(Value::String(value.clone()))
            .map_err(|_| ModelCacheError::InvalidCacheBiasConfiguration(value.clone()))?;

        Ok(CacheBias::KeepCachedFor(duration))
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
        Ok(path.file_name().and_then(|s| s.to_str()).map(|s| s.into()))
    }

    async fn get_marker_path(
        &self,
        entry: &str,
        config: &ModelCacheConfiguration,
    ) -> Result<String> {
        Ok("".into())
    }

    fn is_expired(
        &self,
        entry: &str,
        config: &ModelCacheConfiguration,
        _marker: &(),
        marker_metadata: Metadata,
    ) -> Result<bool> {
        config.is_expired(
            &RenderRequestEntry::TextureHash(entry.to_string()),
            marker_metadata
        )
    }

    async fn write_cache(
        &self,
        entry: &str,
        value: &MojangTexture,
        _config: &ModelCacheConfiguration,
        file: &PathBuf,
    ) -> Result<()> {
        fs::write(file, value.data())
            .await
            .explain(format!("Unable to write texture {:?} to cache", entry))?;

        Ok(())
    }

    async fn read_cache(
        &self,
        entry: &str,
        _config: &ModelCacheConfiguration,
        file: &PathBuf,
        _marker: &(),
    ) -> Result<Option<MojangTexture>> {
        if !file.exists() {
            return Ok(None);
        }

        let data = fs::read(file)
            .await
            .explain(format!("Unable to read texture {:?} from cache", entry))?;

        Ok(Some(MojangTexture::new_named(entry.to_string(), data)))
    }

    async fn read_marker(
        &self,
        _entry: &str,
        _config: &ModelCacheConfiguration,
        _marker: &PathBuf,
    ) -> Result<()> {
        Ok(())
    }

    async fn write_marker(
        &self,
        _entry: &str,
        _value: &MojangTexture,
        _config: &ModelCacheConfiguration,
        _marker: &PathBuf,
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

        return Ok(Self {
            mojang: mojang.clone(),
            resolved_textures: resolved,
        });
    }

    pub async fn get_cached_texture(&self, texture_id: &str) -> Result<Option<MojangTexture>> {
        self.mojang.get_cached_entry(&texture_id).await
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
            RenderRequestEntry::MojangPlayerUuid(u) | RenderRequestEntry::GeyserPlayerUuid(u) => {
                Some(u.to_string())
            }
            RenderRequestEntry::TextureHash(_) | RenderRequestEntry::PlayerSkin(_) => None,
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
        config.is_expired(entry, marker_metadata)
    }

    async fn write_cache(
        &self,
        entry: &RenderRequestEntry,
        value: &ResolvedRenderEntryTextures,
        _config: &ModelCacheConfiguration,
        base: &PathBuf,
    ) -> Result<()> {
        if !base.exists() {
            fs::create_dir_all(base)
                .await
                .explain(format!("Unable to create cache directory for {:?}", &entry))?;
        }

        for (texture_type, texture) in &value.textures {
            let texture_path = base.join(format!("{}{}", Into::<&str>::into(texture_type), ".png"));

            if let Some(texture_hash) = texture.hash() {
                let cache_path = self
                    .mojang_texture_cache
                    .set_cache_entry(&texture_hash.as_str(), &texture)
                    .await?;

                if let Some(cache_path) = cache_path {
                    let cache_path = cache_path.canonicalize().explain(format!(
                        "Unable to canonicalize cache path for texture {:?} for {:?}",
                        texture_hash, entry
                    ))?;

                    symlink::symlink_file(cache_path, texture_path).explain(format!(
                        "Unable to create symlink for texture {:?} for {:?}",
                        texture_hash, entry
                    ))?;
                }
            }
        }

        Ok(())
    }

    async fn read_cache(
        &self,
        entry: &RenderRequestEntry,
        _config: &ModelCacheConfiguration,
        base: &PathBuf,
        marker: &[u8; 1],
    ) -> Result<Option<ResolvedRenderEntryTextures>> {
        let mut textures = HashMap::new();

        for texture in ResolvedRenderEntryTextureType::iter() {
            let texture_path = base.join(format!("{}{}", Into::<&str>::into(&texture), ".png"));

            if texture_path.exists() {
                let read = fs::read(texture_path).await.explain(format!(
                    "Unable to read texture {:?} for {:?}",
                    &texture, &entry
                ))?;

                textures.insert(texture, MojangTexture::new_unnamed(read));
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
        marker: &PathBuf,
    ) -> Result<[u8; 1]> {
        let result = fs::read(marker)
            .await
            .explain(format!("Unable to read marker file for {:?}", entry))?;

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
        marker: &PathBuf,
    ) -> Result<()> {
        fs::write(marker, value.to_marker_slice())
            .await
            .explain(format!("Unable to write marker file for {:?}", entry))?;

        Ok(())
    }
}
