use std::{
    fs::{self, Metadata},
    marker::PhantomData,
    path::PathBuf,
};

use async_trait::async_trait;
use derive_more::Debug;
use tracing::{instrument, trace};

use crate::error::{ExplainableExt, Result};

pub struct CacheSystem<Key, ResultEntry, Config, Marker, Handler>
where
    Key: Debug + ?Sized,
    Handler: CacheHandler<Key, ResultEntry, Config, Marker> + Sync,
{
    base_path: PathBuf,
    config: Config,
    handler: Handler,
    _phantom: PhantomData<(ResultEntry, Marker, Key)>,
}

#[async_trait]
#[allow(unused_variables)]
pub trait CacheHandler<Key, Value, Config, Marker>
where
    Key: ?Sized,
{
    /// Gets the cache key for the given entry.
    ///
    /// If the entry is not cached, this should return `None`.
    async fn get_cache_key(&self, entry: &Key, config: &Config) -> Result<Option<String>>;

    /// Checks whether the given entry is expired.
    ///
    /// If the entry is expired, it will be removed from the cache if it exists.
    fn is_expired(
        &self,
        entry: &Key,
        config: &Config,
        marker: &Marker,
        marker_metadata: Metadata,
    ) -> Result<bool>;

    /// Writes the given entry to the cache.
    async fn write_cache(
        &self,
        entry: &Key,
        value: &Value,
        config: &Config,
        file: &PathBuf,
    ) -> Result<()>;

    /// Reads the given entry from the cache.
    async fn read_cache(
        &self,
        entry: &Key,
        config: &Config,
        file: &PathBuf,
        marker: &Marker,
    ) -> Result<Option<Value>>;

    /// Returns the path to the marker file.
    async fn get_marker_path(&self, entry: &Key, config: &Config) -> Result<String> {
        Ok("marker".into())
    }

    /// Read the marker file for the given entry.
    ///
    /// The marker file is used to denote when the entry was cached.
    /// It can be empty, but it must exist.
    async fn read_marker(&self, entry: &Key, config: &Config, marker: &PathBuf) -> Result<Marker>;

    /// Writes the marker file for the given entry.
    ///
    /// The marker file is used to denote when the entry was cached.
    /// It can be empty, but it must exist.
    async fn write_marker(
        &self,
        entry: &Key,
        value: &Value,
        config: &Config,
        marker: &PathBuf,
    ) -> Result<()>;

    /// Whether to always overwrite the cache entry if it exists.
    fn always_overwrite(&self) -> bool {
        false
    }
}

impl<Key, ResultEntry, Config, Marker, Handler>
    CacheSystem<Key, ResultEntry, Config, Marker, Handler>
where
    Key: Debug + ?Sized,
    Handler: CacheHandler<Key, ResultEntry, Config, Marker> + Sync,
{
    pub fn new(base_path: PathBuf, config: Config, handler: Handler) -> Result<Self> {
        fs::create_dir_all(&base_path)
            .explain(format!("Unable to create cache directory {:?}", &base_path))?;

        Ok(Self {
            base_path,
            config,
            handler,
            _phantom: PhantomData,
        })
    }

    pub async fn get_cache_entry_path(&self, entry: &Key) -> Result<Option<PathBuf>> {
        let key = self.handler.get_cache_key(entry, &self.config).await?;

        Ok(key.map(|k| self.base_path.join(k)))
    }

    #[instrument(skip(self))]
    pub async fn get_cached_entry(&self, entry: &Key) -> Result<Option<ResultEntry>> {
        let path = self.get_cache_entry_path(entry).await?;

        if let Some(path) = path {
            if !path.exists() {
                trace!("Cache entry path doesn't exist.");
                return Ok(None);
            }
            
            let marker_path = self.handler.get_marker_path(entry, &self.config).await?;
            let marker_path = path.join(marker_path);
            
            if !marker_path.exists() {
                trace!("Cache entry path {} doesn't exist.", marker_path.display());
                return Ok(None);
            }

            let marker = self
                .handler
                .read_marker(entry, &self.config, &marker_path)
                .await?;

            let marker_metadata = marker_path
                .metadata()
                .explain(format!("Unable to read marker for entry {:?} ({})", entry, marker_path.display()))?;

            let is_expired = self
                .handler
                .is_expired(entry, &self.config, &marker, marker_metadata)?;

            if is_expired {
                trace!("Entry is expired, discarding.");
                if path.is_dir() {
                    fs::remove_dir_all(path)
                        .explain(format!("Unable to remove expired cache entry {:?}", entry))?;
                } else {
                    fs::remove_file(path)
                        .explain(format!("Unable to remove expired cache entry {:?}", entry))?;
                }

                return Ok(None);
            }

            let result = self
                .handler
                .read_cache(entry, &self.config, &path, &marker)
                .await?;

            trace!("Cache entry found.");
            
            Ok(result)
        } else {
            Ok(None)
        }
    }

    pub async fn set_cache_entry(
        &self,
        entry: &Key,
        value: &ResultEntry,
    ) -> Result<Option<PathBuf>> {
        let path = self.get_cache_entry_path(entry).await?;

        if let Some(path) = &path {
            if path.exists() && !self.handler.always_overwrite() {
                return Ok(Some(path.clone()));
            }

            let marker_path = self.handler.get_marker_path(entry, &self.config).await?;
            let marker_path = path.join(marker_path);

            self.handler
                .write_cache(entry, value, &self.config, &path)
                .await?;

            self.handler
                .write_marker(entry, value, &self.config, &marker_path)
                .await?;
        }

        Ok(path)
    }
}
