use std::{collections::HashMap, path::PathBuf, time::Duration, marker::PhantomData};

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use tracing::instrument;

use crate::{
    config::ModelCacheConfiguration,
    model::resolver::{MojangTexture, ResolvedRenderEntryTextureType, ResolvedRenderEntryTextures},
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum CacheBias {
    KeepCachedFor(Duration),
    CacheIndefinitely,
}

pub(crate) struct ModelCache {
    cache_path: PathBuf,
    cache_config: ModelCacheConfiguration,
}

use std::{fs, time::SystemTime};

use crate::error::{ExplainableExt, ModelCacheError, Result};

use super::entry::RenderRequestEntry;

struct CacheSystem<Entry, Config> {
    config: Config,
    _phantom: PhantomData<Entry>,
}

impl ModelCache {
    pub(crate) fn new(cache_path: PathBuf, cache_config: ModelCacheConfiguration) -> Result<Self> {
        let cache = Self {
            cache_path,
            cache_config,
        };

        cache.init_dirs()?;

        Ok(cache)
    }

    fn as_key(k: &RenderRequestEntry) -> Option<String> {
        match k {
            RenderRequestEntry::PlayerUuid(u) => Some(u.to_string()),
            RenderRequestEntry::TextureHash(h) => Some(h.to_string()),
            RenderRequestEntry::PlayerSkin(_) => None,
        }
    }

    fn get_entry_texture_path(
        &self,
        entry: &RenderRequestEntry,
        texture: &ResolvedRenderEntryTextureType,
    ) -> PathBuf {
        self.get_base_path(&entry)
            .map(|p| p.join(format!("{}{}", Into::<&str>::into(texture), ".png")))
            .unwrap()
    }

    fn get_base_texture_path(&self) -> PathBuf {
        self.cache_path.join("textures")
    }

    fn get_texture_path(&self, name: &str) -> PathBuf {
        self.get_base_texture_path()
            .join(format!("{}{}", Into::<&str>::into(name), ".png"))
    }

    fn get_base_resolved_path(&self) -> PathBuf {
        self.cache_path.join("resolved")
    }

    fn get_base_path(&self, k: &RenderRequestEntry) -> Option<PathBuf> {
        Self::as_key(k).map(|p| self.get_base_resolved_path().join(p))
    }

    fn marker_path(&self, k: &RenderRequestEntry) -> Option<PathBuf> {
        self.get_base_path(k).map(|p| p.join("marker"))
    }

    fn is_expired(&self, k: &RenderRequestEntry) -> Result<bool> {
        let bias = self.cache_config.cache_biases.get(k);

        let duration = if let Some(bias) = bias {
            match bias {
                CacheBias::KeepCachedFor(duration) => duration,
                CacheBias::CacheIndefinitely => &Duration::MAX,
            }
        } else {
            &self.cache_config.resolve_cache_duration
        };

        // Short-circuit never expiring entry.
        if duration == &Duration::MAX {
            return Ok(false);
        }

        let marker_path = self.marker_path(k);

        if let Some(marker_path) = marker_path {
            // Our marker doesn't exist, expire the cache because something went wrong when writing to the cache
            if !marker_path.exists() {
                return Ok(true);
            }

            let expiry = marker_path
                .metadata()
                .and_then(|m| m.modified())
                .map_err(|_| ModelCacheError::MarkerMetadataError(k.clone()))?
                + *duration;

            return Ok(expiry < SystemTime::now());
        }

        Ok(false)
    }

    pub(crate) fn get_cached_texture(&self, name: &str) -> Result<Option<MojangTexture>> {
        let path = self.get_texture_path(&name);

        if path.exists() {
            let data =
                fs::read(path).explain(format!("Unable to read cached texture {:?}", name))?;

            Ok(Some(MojangTexture::new_named(name.to_owned(), data)))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn cache_texture(&self, data: &[u8], name: &str) -> Result<PathBuf> {
        let texture_path = self.get_texture_path(&name);

        if !texture_path.exists() {
            fs::write(&texture_path, data)
                .explain(format!("Unable to write texture {:?} to cache", name))?;
        }

        Ok(texture_path)
    }

    #[instrument(skip(self))]
    pub(crate) fn get_cached_resolved_entity(
        &self,
        k: &RenderRequestEntry,
    ) -> Result<Option<ResolvedRenderEntryTextures>> {
        let base_path = self.get_base_path(k);
        if let Some(base_path) = base_path {
            if !base_path.exists() {
                return Ok(None);
            }

            if self.is_expired(k)? {
                self.cache_remove(k)?;
                return Ok(None);
            }

            let marker_path = self.marker_path(&k);
            if marker_path.is_none() {
                return Ok(None);
            }
            let marker_path = marker_path.unwrap();
            let marker =
                fs::read(marker_path).explain(format!("Unable to read marker file for {:?}", k))?;

            let mut textures: HashMap<ResolvedRenderEntryTextureType, MojangTexture> =
                HashMap::new();

            for texture in ResolvedRenderEntryTextureType::iter() {
                let texture_path = self.get_entry_texture_path(k, &texture);

                if texture_path.exists() {
                    let read = fs::read(texture_path)
                        .explain(format!("Unable to read texture {:?} for {:?}", texture, k))?;
                    textures.insert(texture, MojangTexture::new_unnamed(read));
                }
            }

            Ok(Some(ResolvedRenderEntryTextures::new_from_marker_slice(
                textures, &marker,
            )))
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn cache_resolved_entity(
        &self,
        k: &RenderRequestEntry,
        v: ResolvedRenderEntryTextures,
    ) -> Result<ResolvedRenderEntryTextures> {
        let base_path = self.get_base_path(k);

        if let Some(base_path) = base_path {
            fs::create_dir_all(base_path)
                .explain(format!("Unable to create cache directory for {:?}", k))?;

            // Write our marker file to denote when we cached this entry
            let marker_path = self.marker_path(&k);
            if let Some(path) = marker_path {
                fs::write(path, v.to_marker_slice())
                    .explain(format!("Unable to write marker file for {:?}", k))?;
            }

            for (tex_type, texture) in v.textures.iter() {
                let entry_texture_path = self.get_entry_texture_path(k, tex_type);
                if let Some(hash) = &texture.hash {
                    let cache_path = self.cache_texture(&texture.data, hash)?;

                    if !entry_texture_path.exists() {
                        let cache_path = cache_path
                            .canonicalize()
                            .explain(format!("Unable to canonicalize cache path for {:?}", hash))?;

                        symlink::symlink_file(cache_path, entry_texture_path).explain(format!(
                            "Unable to create symlink for texture {:?} for {:?}",
                            hash, k
                        ))?;
                    }
                } else {
                    return Err(ModelCacheError::InvalidRequestCacheAttempt(
                        "Received texture with no hash".to_owned(),
                    )
                    .into());
                }
            }
        }

        Ok(v)
    }

    pub(crate) fn cache_remove(
        &self,
        k: &RenderRequestEntry,
    ) -> Result<Option<ResolvedRenderEntryTextures>> {
        let base_path = self.get_base_path(&k);

        if let Some(path) = base_path {
            if path.exists() {
                fs::remove_dir_all(path)
                    .explain(format!("Unable to remove cache directory for {:?}", k))?;
            }
        }

        Ok(None)
    }

    fn init_dirs(&self) -> Result<()> {
        let paths = vec![
            self.cache_path.to_owned(),
            self.get_base_resolved_path(),
            self.get_base_texture_path(),
        ];

        for path in paths {
            fs::create_dir_all(&path)
                .explain(format!("Unable to create cache directory {:?}", &path))?;
        }

        Ok(())
    }
}
