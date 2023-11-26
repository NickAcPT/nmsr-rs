use std::{
    borrow::Cow,
    fs::Metadata,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use derive_more::Debug;
use tracing::{instrument};

use crate::error::{Result};

pub struct CacheSystem;

#[async_trait]
#[allow(unused_variables)]
pub trait CacheHandler<Key, Value, Config, Marker>
where
    Key: ToOwned + ?Sized,
{
    /// Given a path, returns the cache key for the entry.
    /// This is used to determine which entries are expired when cleaning the cache.
    async fn read_key_from_path<'a>(
        &'a self,
        config: &Config,
        base_path: &'a Path,
    ) -> Result<Option<Cow<'a, Key>>>;

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
        file: &Path,
    ) -> Result<()>;

    /// Reads the given entry from the cache.
    async fn read_cache(
        &self,
        entry: &Key,
        config: &Config,
        file: &Path,
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
    async fn read_marker(&self, entry: &Key, config: &Config, marker: &Path) -> Result<Marker>;

    /// Writes the marker file for the given entry.
    ///
    /// The marker file is used to denote when the entry was cached.
    /// It can be empty, but it must exist.
    async fn write_marker(
        &self,
        entry: &Key,
        value: &Value,
        config: &Config,
        marker: &Path,
    ) -> Result<()>;

    /// Whether to always overwrite the cache entry if it exists.
    fn always_overwrite(&self) -> bool {
        false
    }
}
