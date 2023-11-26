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
    mojang_texture_cache: Arc<CacheSystem,>,
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
        Ok(())
    }

    async fn read_cache(
        &self,
        entry: &str,
        config: &ModelCacheConfiguration,
        file: &Path,
        _marker: &(),
    ) -> Result<Option<MojangTexture>> 
    {
        Ok(None)
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
        CacheSystem,
    >,
    resolved_textures: CacheSystem
}

impl ModelCache {
    pub async fn new(cache_path: PathBuf, cache_config: ModelCacheConfiguration) -> Result<Self> {
        let mojang = CacheSystem;

        let mojang = Arc::new(mojang);

        let resolved = CacheSystem;

        Ok(Self {
            mojang: mojang.clone(),
            resolved_textures: resolved,
        })
    }

    pub async fn get_cached_texture(&self, texture_id: &str) -> Result<Option<MojangTexture>> {
        Ok(None)
    }

    pub async fn cache_texture(&self, texture: &MojangTexture) -> Result<()> {
        Ok(())
    }

    pub async fn get_cached_resolved_texture(
        &self,
        entry: &RenderRequestEntry,
    ) -> Result<Option<ResolvedRenderEntryTextures>> {
        Ok(None)
    }

    pub async fn cache_resolved_texture(
        &self,
        entry: &RenderRequestEntry,
        textures: &ResolvedRenderEntryTextures,
    ) -> Result<()> {
        Ok(())
    }

    pub(crate) async fn do_cache_clean_up(&self) -> Result<()> {
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
            RenderRequestEntry::TextureHash(hash) => Some(hash.clone()),
            RenderRequestEntry::PlayerSkin(_) => None,
        })
    }

    #[inline]
    async fn read_key_from_path<'a>(
        &'a self,
        _config: &ModelCacheConfiguration,
        path: &'a Path,
    ) -> Result<Option<Cow<'a, RenderRequestEntry>>> { Ok(None) }

    fn is_expired(
        &self,
        entry: &RenderRequestEntry,
        config: &ModelCacheConfiguration,
        _marker: &[u8; 1],
        marker_metadata: Metadata,
    ) -> Result<bool> {
        Ok((false))
    }

    async fn write_cache(
        &self,
        entry: &RenderRequestEntry,
        value: &ResolvedRenderEntryTextures,
        _config: &ModelCacheConfiguration,
        base: &Path,
    ) -> Result<()> {
        Ok(())
    }

    async fn read_cache(
        &self,
        entry: &RenderRequestEntry,
        config: &ModelCacheConfiguration,
        base: &Path,
        marker: &[u8; 1],
    ) -> Result<Option<ResolvedRenderEntryTextures>> 
    {
        Ok(None)
    }

    async fn read_marker(
        &self,
        entry: &RenderRequestEntry,
        _config: &ModelCacheConfiguration,
        marker: &Path,
    ) -> Result<[u8; 1]> {
        Ok([0])
    }

    async fn write_marker(
        &self,
        entry: &RenderRequestEntry,
        value: &ResolvedRenderEntryTextures,
        _config: &ModelCacheConfiguration,
        marker: &Path,
    ) -> Result<()> {
        Ok(())
    }
}
