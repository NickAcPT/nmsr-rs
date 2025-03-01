use std::{path::PathBuf, sync::Arc, time::Duration};

use super::entry::RenderRequestEntry;
use crate::{
    error::{ModelCacheError, ModelCacheResult, Result},
    model::request::cache::{
        entries::ResolvedModelTexturesCacheHandler, names::MojangNamesCacheHandler,
        textures::MojangTextureCacheHandler,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;
use uuid::Uuid;

pub(crate) mod entries;
pub(crate) mod names;
pub(crate) mod textures;

use crate::{
    caching::CacheSystem,
    config::ModelCacheConfiguration,
    model::resolver::{MojangTexture, ResolvedRenderEntryTextures},
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

pub struct ModelCache {
    mojang_textures: Arc<
        CacheSystem<str, MojangTexture, ModelCacheConfiguration, (), MojangTextureCacheHandler>,
    >,
    resolved_names: CacheSystem<str, Uuid, ModelCacheConfiguration, (), MojangNamesCacheHandler>,
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

        let names = CacheSystem::new(
            cache_path.join("names"),
            cache_config.clone(),
            MojangNamesCacheHandler,
        )
        .await?;

        Ok(Self {
            mojang_textures: mojang.clone(),
            resolved_names: names,
            resolved_textures: resolved,
        })
    }

    pub async fn get_cached_texture(&self, texture_id: &str) -> Result<Option<MojangTexture>> {
        self.mojang_textures.get_cached_entry(texture_id).await
    }

    pub async fn cache_texture(&self, texture: &MojangTexture) -> Result<()> {
        if let Some(hash) = texture.hash() {
            self.mojang_textures
                .set_cache_entry(hash, texture)
                .await
                .map(|_| ())
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

    pub async fn cache_resolved_name(&self, name: &str, uuid: Uuid) -> Result<()> {
        self.resolved_names
            .set_cache_entry(name, &uuid)
            .await
            .map(|_| ())
    }

    pub async fn get_cached_resolved_name(&self, name: &str) -> Result<Option<Uuid>> {
        self.resolved_names.get_cached_entry(name).await
    }

    pub(crate) async fn do_cache_clean_up(&self) -> Result<()> {
        self.resolved_textures.perform_cache_cleanup().await?;
        self.mojang_textures.perform_cache_cleanup().await?;

        Ok(())
    }
}
