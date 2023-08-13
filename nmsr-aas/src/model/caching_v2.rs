use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    time::{Duration, SystemTime}, fmt::format,
};

use crate::utils::Result;
use strum::IntoEnumIterator;

#[cfg(feature = "tracing")]
use tracing::instrument;

use crate::{model::resolver::RenderEntryTextureType, utils::errors::NMSRaaSError};

use super::{
    resolver::{MojangTexture, ResolvedRenderEntryTextures},
    RenderRequestEntry,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) enum CacheBias {
    KeepCachedFor(Duration),
    CacheIndefinitely,
}

pub(crate) struct ModelCacheConfiguration {
    /// The duration of time to keep a resolved model in the cache.
    /// This is effectively for how long to cache the player's skin, cape and other textures.
    /// When given a player uuid, we will resolve it with Mojang's API and cache the result.
    pub(crate) model_resolve_cache_duration: Duration,
}
pub(crate) struct ModelCache {
    biases: HashMap<RenderRequestEntry, CacheBias>,
    cache_path: PathBuf,
    cache_config: ModelCacheConfiguration,
}

impl ModelCache {
    pub(crate) fn new(
        biases: HashMap<RenderRequestEntry, CacheBias>,
        cache_path: PathBuf,
        cache_config: ModelCacheConfiguration,
    ) -> Self {
        let cache = Self {
            biases,
            cache_path,
            cache_config,
        };

        cache
            .init_dirs()
            .expect("Failed to initialize cache directories");

        cache
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
        texture: &RenderEntryTextureType,
    ) -> PathBuf {
        self.get_base_path(&entry)
            .map(|p| p.join(format!("{}{}", Into::<&str>::into(texture), ".png")))
            .unwrap()
    }

    fn get_base_texture_path(&self) -> PathBuf {
        self.cache_path.join("textures")
    }

    fn get_texture_path(&self, name: &str) -> PathBuf {
        self.get_base_texture_path().join(format!("{}{}", Into::<&str>::into(name), ".png"))
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
        let bias = self.biases.get(k);

        let duration = if let Some(bias) = bias {
            match bias {
                CacheBias::KeepCachedFor(duration) => duration,
                CacheBias::CacheIndefinitely => &Duration::MAX,
            }
        } else {
            &self.cache_config.model_resolve_cache_duration
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

            let expiry = marker_path.metadata()?.modified()? + *duration;

            return Ok(expiry < SystemTime::now());
        }

        Ok(false)
    }

    pub(crate) fn get_cached_texture(&self, name: &str) -> Result<Option<MojangTexture>> {
        let path = self.get_texture_path(&name);

        if path.exists() {
            let data = fs::read(path)?;

            Ok(Some(MojangTexture::new_named(name.to_owned(), data)))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn cache_texture(&self, data: &[u8], name: &str) -> Result<PathBuf> {
        let texture_path = self.get_texture_path(&name);

        if !texture_path.exists() {
            fs::write(&texture_path, data)?;
        }

        Ok(texture_path)
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self)))]
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
            let marker = fs::read(marker_path)?;
            
                        

            let mut textures: HashMap<RenderEntryTextureType, MojangTexture> = HashMap::new();

            for texture in RenderEntryTextureType::iter() {
                let texture_path = self.get_entry_texture_path(k, &texture);

                if texture_path.exists() {
                    textures.insert(texture, MojangTexture::new_unnamed(fs::read(texture_path)?));
                }
            }

            Ok(Some(ResolvedRenderEntryTextures::new_from_marker_slice(textures, &marker)))
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
            fs::create_dir_all(base_path)?;

            // Write our marker file to denote when we cached this entry
            let marker_path = self.marker_path(&k);
            if let Some(path) = marker_path {
                fs::write(path, v.to_marker_slice())?;
            }

            for (tex_type, texture) in v.textures.iter() {
                let entry_texture_path = self.get_entry_texture_path(k, tex_type);
                if let Some(hash) = &texture.hash {
                    let cache_path = self.cache_texture(&texture.data, hash)?;

                    if !entry_texture_path.exists() {
                        symlink::symlink_file(cache_path.canonicalize()?, entry_texture_path.canonicalize()?)?;
                    }
                } else {
                    return Err(NMSRaaSError::InvalidPlayerRequest(
                        "Received texture with no hash".to_owned(),
                    ));
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
                fs::remove_dir_all(path)?;
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
            fs::create_dir_all(path)?;
        }

        Ok(())
    }
}
