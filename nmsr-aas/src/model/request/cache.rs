use std::{
    collections::HashMap,
    fs::Metadata,
    marker::PhantomData,
    path::PathBuf,
    time::Duration,
};

use async_trait::async_trait;
use derive_more::Deref;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use tracing::instrument;

use crate::{
    config::ModelCacheConfiguration,
    error::ModelCacheResult,
    model::resolver::{MojangTexture, ResolvedRenderEntryTextureType, ResolvedRenderEntryTextures},
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum CacheBias {
    KeepCachedFor(Duration),
    CacheIndefinitely,
}

pub(crate) struct ModelTexturesCacheHandler;

#[derive(Deref)]
#[repr(transparent)]
pub(crate) struct ModelCache {
    cache: CacheSystem<
        RenderRequestEntry,
        ResolvedRenderEntryTextures,
        ModelCacheConfiguration,
        (),
        ModelTexturesCacheHandler,
    >,
}

use std::{fs, time::SystemTime};

use crate::error::{ExplainableExt, ModelCacheError, Result};

use super::entry::RenderRequestEntry;

pub struct CacheSystem<
    Key,
    ResultEntry,
    Config,
    Marker,
    Handler: CacheHandler<Key, ResultEntry, Config, Marker>,
> {
    config: Config,
    handler: Handler,
    _phantom: PhantomData<(Key, ResultEntry, Marker)>,
}

#[async_trait]
pub trait CacheHandler<Key, Value, Config, Marker> {
    /// Gets the cache key for the given entry.
    ///
    /// If the entry is not cached, this should return `None`.
    async fn get_cache_key(&self, entry: &Key, config: &Config) -> Result<Option<String>>;

    /// Checks whether the given entry is expired.
    ///
    /// If the entry is expired, it will be removed from the cache if it exists.
    async fn is_expired(
        &self,
        entry: &Key,
        config: &Config,
        marker: Marker,
        marker_metadata: Metadata,
    ) -> Result<bool>;

    /// Writes the given entry to the cache.
    async fn write_cache(
        &self,
        entry: &Key,
        value: &Value,
        config: &Config,
        file: PathBuf,
    ) -> Result<()>;

    /// Reads the given entry from the cache.
    async fn read_cache(
        &self,
        entry: &Key,
        config: &Config,
        file: PathBuf,
        marker: Marker,
    ) -> Result<Option<Value>>;

    /// Read the marker file for the given entry.
    ///
    /// The marker file is used to denote when the entry was cached.
    /// It can be empty, but it must exist.
    async fn read_marker(&self, entry: &Key, config: &Config, marker: PathBuf) -> Result<Marker>;

    /// Writes the marker file for the given entry.
    ///
    /// The marker file is used to denote when the entry was cached.
    /// It can be empty, but it must exist.
    async fn write_marker(
        &self,
        entry: &Key,
        value: &Value,
        config: &Config,
        marker: PathBuf,
    ) -> Result<()>;
}

impl ModelCache {
    pub(crate) fn new(cache_path: PathBuf, cache_config: ModelCacheConfiguration) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl CacheHandler<RenderRequestEntry, ResolvedRenderEntryTextures, ModelCacheConfiguration, ()>
    for ModelTexturesCacheHandler
{
    async fn get_cache_key(
        &self,
        entry: &RenderRequestEntry,
        _config: &ModelCacheConfiguration,
    ) -> Result<Option<String>> {
        Ok(match entry {
            RenderRequestEntry::PlayerUuid(u) => Some(u.to_string()),
            RenderRequestEntry::TextureHash(h) => Some(h.to_string()),
            RenderRequestEntry::PlayerSkin(_) => None,
        })
    }
    async fn is_expired(
        &self,
        entry: &RenderRequestEntry,
        config: &ModelCacheConfiguration,
        marker: (),
        marker_metadata: Metadata,
    ) -> Result<bool> {
        let bias = config.cache_biases.get(entry);

        let duration = if let Some(bias) = bias {
            match bias {
                CacheBias::KeepCachedFor(duration) => duration,
                CacheBias::CacheIndefinitely => &Duration::MAX,
            }
        } else {
            &config.resolve_cache_duration
        };

        // Short-circuit never expiring entry.
        if duration == &Duration::MAX {
            return Ok(false);
        }

        let expiry = marker_metadata.modified().explain(format!(
            "Unable to get marker modified date for entry {:?}",
            &entry
        ))? + *duration;

        return Ok(expiry < SystemTime::now());
    }

    async fn write_cache(
        &self,
        entry: &RenderRequestEntry,
        value: &ResolvedRenderEntryTextures,
        config: &ModelCacheConfiguration,
        file: PathBuf,
    ) -> Result<()> {
        Ok(())
    }

    async fn read_cache(
        &self,
        entry: &RenderRequestEntry,
        config: &ModelCacheConfiguration,
        base: PathBuf,
        marker: (),
    ) -> Result<Option<ResolvedRenderEntryTextures>> {
        let mut textures = HashMap::new();

        for texture in ResolvedRenderEntryTextureType::iter() {
            let mut texture_path = base.clone();
            texture_path.push(format!("{}{}", Into::<&str>::into(&texture), ".png"));

            if texture_path.exists() {
                let read = fs::read(texture_path).explain(format!(
                    "Unable to read texture {:?} for {:?}",
                    &texture, &entry
                ))?;

                textures.insert(texture, MojangTexture::new_unnamed(read));
            }
        }

        let marker = [3];

        Ok(Some(ResolvedRenderEntryTextures::new_from_marker_slice(
            textures, &marker,
        )))
    }

    async fn read_marker(
        &self,
        entry: &RenderRequestEntry,
        config: &ModelCacheConfiguration,
        marker: PathBuf,
    ) -> Result<()> {
        // TODO: Read marker file
        Ok(())
    }

    async fn write_marker(
        &self,
        entry: &RenderRequestEntry,
        value: &ResolvedRenderEntryTextures,
        config: &ModelCacheConfiguration,
        marker: PathBuf,
    ) -> Result<()> {
        fs::write(marker, value.to_marker_slice())
            .explain(format!("Unable to write marker file for {:?}", entry))?;

        Ok(())
    }
}