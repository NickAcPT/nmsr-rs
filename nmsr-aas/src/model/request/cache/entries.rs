use std::{borrow::Cow, collections::BTreeMap, fs::Metadata, path::Path, sync::Arc};

use async_trait::async_trait;
use tokio::fs;
use tracing::trace;

use crate::{
    caching::{CacheHandler, CacheSystem},
    config::ModelCacheConfiguration,
    error::{ExplainableExt, ModelCacheError, Result},
    model::{
        request::{cache::textures::MojangTextureCacheHandler, entry::RenderRequestEntry},
        resolver::{MojangTexture, ResolvedRenderEntryTextureType, ResolvedRenderEntryTextures},
    },
};

#[cfg(feature = "ears")]
use crate::model::resolver::ResolvedRenderEntryEarsTextureType;
pub struct ResolvedModelTexturesCacheHandler {
    pub mojang_texture_cache: Arc<
        CacheSystem<str, MojangTexture, ModelCacheConfiguration, (), MojangTextureCacheHandler>,
    >,
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
            RenderRequestEntry::MojangPlayerUuid(u)
            | RenderRequestEntry::MojangOfflinePlayerUuid(u)
            | RenderRequestEntry::GeyserPlayerUuid(u) => Some(u.to_string()),
            RenderRequestEntry::TextureHash(hash)
            | RenderRequestEntry::DefaultSkinTextureHash(hash) => Some(hash.clone()),
            RenderRequestEntry::MojangPlayerName(_) | RenderRequestEntry::PlayerSkin(_, _) => None,
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
            .and_then(|p| p.to_str())
            .unwrap_or_default()
            .to_string();

        Ok(RenderRequestEntry::try_from(file_name).map(|e| Cow::Owned(e)).ok())
    }

    fn is_expired(
        &self,
        entry: &RenderRequestEntry,
        config: &ModelCacheConfiguration,
        _marker: &[u8; 1],
        marker_metadata: Metadata,
    ) -> Result<bool> {
        config.is_expired(entry, &marker_metadata)
    }

    async fn write_cache(
        &self,
        entry: &RenderRequestEntry,
        value: &ResolvedRenderEntryTextures,
        _config: &ModelCacheConfiguration,
        base: &Path,
    ) -> Result<()> {
        if !base.exists() {
            fs::create_dir_all(base)
                .await
                .explain(format!("Unable to create cache directory for {:?}", &entry))?;
        }

        for (texture_type, texture) in &value.textures {
            let texture_path =
                base.join(format!("{}{}", Into::<&str>::into(*texture_type), ".png"));

            if let Some(texture_hash) = texture.hash() {
                let cache_path = self
                    .mojang_texture_cache
                    .set_cache_entry(texture_hash.as_str(), texture)
                    .await?;

                if let Some(cache_path) = cache_path {
                    let cache_path = cache_path.canonicalize().explain(format!(
                        "Unable to canonicalize cache path for texture {texture_hash:?} for {entry:?}"
                    ))?;

                    symlink::symlink_file(cache_path, texture_path).explain(format!(
                        "Unable to create symlink for texture {texture_hash:?} for {entry:?}"
                    ))?;
                }
            }
        }

        Ok(())
    }

    async fn read_cache(
        &self,
        entry: &RenderRequestEntry,
        config: &ModelCacheConfiguration,
        base: &Path,
        marker: &[u8; 1],
    ) -> Result<Option<ResolvedRenderEntryTextures>> {
        let mut textures = BTreeMap::new();

        #[cfg(not(feature = "ears"))]
        let textures_to_read = [
            ResolvedRenderEntryTextureType::Skin,
            ResolvedRenderEntryTextureType::Cape,
        ];

        #[cfg(feature = "ears")]
        let textures_to_read = [
            ResolvedRenderEntryTextureType::Skin,
            ResolvedRenderEntryTextureType::Cape,
            ResolvedRenderEntryTextureType::Ears(ResolvedRenderEntryEarsTextureType::Wings),
            ResolvedRenderEntryTextureType::Ears(ResolvedRenderEntryEarsTextureType::Cape),
            ResolvedRenderEntryTextureType::Ears(
                ResolvedRenderEntryEarsTextureType::EmissiveProcessedSkin,
            ),
            ResolvedRenderEntryTextureType::Ears(
                ResolvedRenderEntryEarsTextureType::EmissiveProcessedWings,
            ),
            ResolvedRenderEntryTextureType::Ears(ResolvedRenderEntryEarsTextureType::EmissiveSkin),
            ResolvedRenderEntryTextureType::Ears(ResolvedRenderEntryEarsTextureType::EmissiveWings),
        ];

        for texture in textures_to_read {
            let is_important_texture = matches!(texture, ResolvedRenderEntryTextureType::Skin);

            let texture_path = base.join(format!("{}{}", Into::<&str>::into(texture), ".png"));

            if texture_path.exists() {
                let read = fs::read(texture_path).await.explain(format!(
                    "Unable to read texture {:?} for {:?}",
                    &texture, &entry
                ))?;

                if is_important_texture && !config.validate_png_data(&read) {
                    trace!("Texture {texture:?} for {entry:?} is invalid, discarding.");
                    CacheSystem::<
                        RenderRequestEntry,
                        ResolvedRenderEntryTextures,
                        ModelCacheConfiguration,
                        [u8; 1],
                        Self,
                    >::invalidate_self(entry, base)
                    .await?;

                    return Ok(None);
                }

                textures.insert(texture, MojangTexture::new_unnamed(read));
            } else if !is_important_texture {
                // If we haven't found a cached texture for an important texture, then we just skip
                continue;
            } else {
                trace!(
                    "Unable to find texture path for important texture {}",
                    texture_path.display()
                );

                // One of the textures has gone missing, this means that this cache entry is invalid and should be removed
                CacheSystem::<
                    RenderRequestEntry,
                    ResolvedRenderEntryTextures,
                    ModelCacheConfiguration,
                    [u8; 1],
                    Self,
                >::invalidate_self(entry, base)
                .await?;
                return Ok(None);
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
        marker: &Path,
    ) -> Result<[u8; 1]> {
        let result = fs::read(marker)
            .await
            .explain(format!("Unable to read marker file for {entry:?}"))?;

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
        marker: &Path,
    ) -> Result<()> {
        fs::write(marker, value.to_marker_slice())
            .await
            .explain(format!("Unable to write marker file for {entry:?}"))?;

        Ok(())
    }
}
