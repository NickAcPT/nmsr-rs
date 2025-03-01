use std::{borrow::Cow, fs::Metadata, path::Path, time::SystemTime};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use tokio::fs;
use tracing::trace;
use uuid::Uuid;
use xxhash_rust::xxh3::xxh3_64;

use crate::{
    caching::{CacheHandler, CacheSystem},
    config::ModelCacheConfiguration,
    error::{ExplainableExt, Result},
};

pub struct MojangNamesCacheHandler;

#[async_trait]
impl CacheHandler<str, Uuid, ModelCacheConfiguration, ()> for MojangNamesCacheHandler {
    async fn read_key_from_path<'a>(
        &'a self,
        _config: &ModelCacheConfiguration,
        path: &'a Path,
    ) -> Result<Option<Cow<'a, str>>> {
        return Ok(path.file_name().map(|p| p.to_string_lossy()));
    }

    async fn get_cache_key(
        &self,
        entry: &str,
        _config: &ModelCacheConfiguration,
    ) -> Result<Option<String>> {
        return Ok(Some(format!("usr_{}", xxh3_64(entry.as_bytes()))));
    }

    fn is_expired(
        &self,
        entry: &str,
        config: &ModelCacheConfiguration,
        _marker: &(),
        marker_metadata: Metadata,
    ) -> Result<bool> {
        let expiry = marker_metadata.modified().explain(format!(
            "Unable to get marker modified date for entry {:?}",
            &entry
        ))? + config.username_cache_duration;

        trace!(
            "Name cache entry expires on {}",
            Into::<DateTime<Local>>::into(expiry)
        );

        Ok(expiry < SystemTime::now())
    }

    async fn write_cache(
        &self,
        entry: &str,
        value: &Uuid,
        _config: &ModelCacheConfiguration,
        file: &Path,
    ) -> Result<()> {
        fs::write(file, value.as_bytes()).await.explain(format!(
            "Unable to write cache file for name entry {entry:?}"
        ))?;

        Ok(())
    }

    async fn read_cache(
        &self,
        entry: &str,
        _config: &ModelCacheConfiguration,
        file: &Path,
        _marker: &(),
    ) -> Result<Option<Uuid>> {
        let result = fs::read(file)
            .await
            .explain(format!("Unable to read marker file for {entry:?}"))?;

        let Ok(uuid) = Uuid::from_slice(&result) else {
            CacheSystem::<str, Uuid, ModelCacheConfiguration, (), Self>::invalidate_self(
                entry, file,
            )
            .await?;

            return Ok(None);
        };

        Ok(Some(uuid))
    }

    async fn get_marker_path(
        &self,
        _entry: &str,
        _config: &ModelCacheConfiguration,
    ) -> Result<String> {
        Ok("".to_string())
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
        _value: &Uuid,
        _config: &ModelCacheConfiguration,
        _marker: &Path,
    ) -> Result<()> {
        Ok(())
    }

    fn always_overwrite(&self) -> bool {
        false
    }
}
