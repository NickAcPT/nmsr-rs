use std::{borrow::Cow, fs::Metadata, path::Path};

use async_trait::async_trait;
use tokio::fs;
use tracing::trace;

use crate::{caching::{CacheHandler, CacheSystem}, config::ModelCacheConfiguration, error::{ExplainableExt, Result}, model::{request::entry::RenderRequestEntry, resolver::MojangTexture}};

pub struct MojangTextureCacheHandler;


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