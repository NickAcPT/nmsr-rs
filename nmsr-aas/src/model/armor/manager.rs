use std::{
    borrow::Cow,
    fs::Metadata,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use tokio::fs;

use crate::{
    caching::{CacheHandler, CacheSystem},
    error::{ArmorManagerError, ExplainableExt, Result},
    utils::http_client::NmsrHttpClient,
};

use super::VanillaMinecraftArmorMaterial;

struct VanillaMinecraftArmorMaterialCacheHandler;

#[async_trait]
impl CacheHandler<VanillaMinecraftArmorMaterial, Vec<u8>, (), ()>
    for VanillaMinecraftArmorMaterialCacheHandler
{
    async fn read_key_from_path<'a>(
        &'a self,
        _config: &(),
        base_path: &'a Path,
    ) -> Result<Option<Cow<'a, VanillaMinecraftArmorMaterial>>> {
        let option = base_path
            .file_name()
            .and_then(|f| f.to_str())
            .map(|f| VanillaMinecraftArmorMaterial::try_from(f))
            .transpose()
            .map_err(ArmorManagerError::ArmorParseError)?;

        Ok(option.map(|f| Cow::Owned(f)))
    }

    async fn get_cache_key(
        &self,
        entry: &VanillaMinecraftArmorMaterial,
        _config: &(),
    ) -> Result<Option<String>> {
        let str: &'static str = entry.into();

        Ok(Some(str.into())).into()
    }

    fn is_expired(
        &self,
        _entry: &VanillaMinecraftArmorMaterial,
        _config: &(),
        _marker: &(),
        _marker_metadata: Metadata,
    ) -> Result<bool> {
        Ok(false)
    }

    async fn write_cache(
        &self,
        entry: &VanillaMinecraftArmorMaterial,
        value: &Vec<u8>,
        _config: &(),
        file: &PathBuf,
    ) -> Result<()> {
        fs::write(file, value)
            .await
            .explain_closure(|| format!("Unable to write armor file for {entry}"))
    }

    async fn read_cache(
        &self,
        entry: &VanillaMinecraftArmorMaterial,
        _config: &(),
        file: &PathBuf,
        _marker: &(),
    ) -> Result<Option<Vec<u8>>> {
        if !file.exists() {
            return Ok(None);
        }

        Ok(Some(fs::read(file).await.explain_closure(|| {
            format!("Unable to read armor file from cache {entry}")
        })?))
    }

    async fn read_marker(
        &self,
        _entry: &VanillaMinecraftArmorMaterial,
        _config: &(),
        _marker: &PathBuf,
    ) -> Result<()> {
        Ok(())
    }

    async fn write_marker(
        &self,
        _entry: &VanillaMinecraftArmorMaterial,
        _value: &Vec<u8>,
        _config: &(),
        _marker: &PathBuf,
    ) -> Result<()> {
        Ok(())
    }
}

pub struct VanillaMinecraftArmorManager {
    client: NmsrHttpClient,
    armor_cache: CacheSystem<
        VanillaMinecraftArmorMaterial,
        Vec<u8>,
        (),
        (),
        VanillaMinecraftArmorMaterialCacheHandler,
    >,
}

impl VanillaMinecraftArmorManager {
    pub async fn new(cache_path: PathBuf) -> Result<Self> {
        let armor_cache = CacheSystem::new(
            cache_path.join("armor"),
            (),
            VanillaMinecraftArmorMaterialCacheHandler,
        )
        .await?;

        Ok(Self {
            client: NmsrHttpClient::new(20),
            armor_cache,
        })
    }
    
    pub async fn get_armor_texture(&self, material: VanillaMinecraftArmorMaterial) -> Result<Vec<u8>> {
        let armor_texture = self
            .armor_cache
            .get_cached_entry(&material).await?;

        if let Some(armor_texture) = armor_texture {
            return Ok(armor_texture);
        }
        
        
        unimplemented!()
    }
}
