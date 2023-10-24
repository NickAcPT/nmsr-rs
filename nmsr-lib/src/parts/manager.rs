use image::RgbaImage;

#[cfg(feature = "parallel_iters")]
use rayon::prelude::*;

use tracing::instrument;
use vfs::VfsPath;

use crate::errors::{NMSRError, Result};
use crate::utils::{into_par_iter_if_enabled, open_image_from_vfs};
use crate::{parts::player_model::PlayerModel, uv::uv_magic::UvImage};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serializable_parts", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serializable_parts_rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct PartsManager {
    pub all_parts: Vec<UvImage>,
    pub model_parts: Vec<UvImage>,
    pub model_overlays: Vec<UvImage>,
    pub environment_background: Option<UvImage>,
    #[cfg(feature = "ears")]
    pub ears_parts_manager: Option<Box<PartsManager>>,
}

impl PartsManager {
    const ENVIRONMENT_BACKGROUND_NAME: &'static str = "environment_background.qoi";

    fn is_part_file(path: &VfsPath) -> Result<bool> {
        let name = path.filename();

        Ok(path.is_file()? && name != PartsManager::ENVIRONMENT_BACKGROUND_NAME)
    }

    #[instrument(level = "trace", skip(root))]
    pub fn new(root: &VfsPath) -> Result<PartsManager> {
        let mut all_parts = Vec::<UvImage>::with_capacity(8);
        let mut model_parts = Vec::<UvImage>::with_capacity(8);
        let mut model_overlays = Vec::<UvImage>::with_capacity(16);

        Self::load_as_parts(root, &mut all_parts, "", false)?;
        Self::load_model_specific_parts(root, &mut model_parts, false)?;

        let overlays_root = root.join("overlays")?;
        if overlays_root.exists()? {
            Self::load_as_parts(&overlays_root, &mut model_overlays, "", true)?;
            Self::load_model_specific_parts(&overlays_root, &mut model_overlays, true)?;
        }

        let environment_background = Self::load_environment_background(root)?;

        Ok(PartsManager {
            all_parts,
            model_parts,
            model_overlays,
            environment_background,
            #[cfg(feature = "ears")]
            ears_parts_manager: Self::load_ears_parts_manager(root)?,
        })
    }

    #[instrument(level = "trace", skip(root, model_parts))]
    fn load_model_specific_parts(
        root: &VfsPath,
        model_parts: &mut Vec<UvImage>,
        store_raw_pixels: bool,
    ) -> Result<()> {
        for model in [PlayerModel::Alex, PlayerModel::Steve].iter() {
            let dir_name = model.get_dir_name();

            let model_parts_dir = root.join(dir_name)?;

            if !model_parts_dir.exists()? {
                continue;
            }

            Self::load_as_parts(&model_parts_dir, model_parts, dir_name, store_raw_pixels)?;
        }

        Ok(())
    }

    #[instrument(level = "trace", skip(dir, parts_map))]
    fn load_as_parts(
        dir: &VfsPath,
        parts_map: &mut Vec<UvImage>,
        path_prefix: &str,
        store_raw_pixels: bool,
    ) -> Result<()> {
        let directory = dir
            .read_dir()
            .map_err(|e| NMSRError::IoError(e, format!("Unable to read {:?}", &dir)))?;

        let mut part_entries = vec![];

        for dir_entry in directory {
            // Skip non part files
            if !Self::is_part_file(&dir_entry)? {
                continue;
            }

            // Compute map entry key
            let name: String = dir_entry
                .filename()
                .chars()
                .take_while(|p| {
                    *p != '.'
                })
                .collect();

            let name = format!("{path_prefix}{name}");

            part_entries.push((name, dir_entry));
        }

        let loaded_parts: Vec<_> = into_par_iter_if_enabled!(part_entries)
            .map(|(name, entry)| Ok((name, open_image_from_vfs(&entry)?)))
            .map(|result: Result<(String, RgbaImage)>| -> Result<UvImage> {
                let (name, image) = result?;
                let uv_image = UvImage::new(name, image, store_raw_pixels);
                Ok(uv_image)
            })
            .collect();
        
        for part in loaded_parts {
            let part = part?;
            parts_map.push(part);
        }
        
        parts_map.sort_by(|a, b| a.name.cmp(&b.name));
        
        Ok(())
    }

    fn load_environment_background(root: &VfsPath) -> Result<Option<UvImage>> {
        let path = &root.join(Self::ENVIRONMENT_BACKGROUND_NAME)?;

        if path.exists()? {
            let image = open_image_from_vfs(path)?;

            Ok(Some(UvImage::new(
                Self::ENVIRONMENT_BACKGROUND_NAME.to_string(),
                image,
                true,
            )))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "ears")]
    fn load_ears_parts_manager(root: &VfsPath) -> Result<Option<Box<PartsManager>>> {
        let ears_dir = root.join("ears")?;
        Ok(if ears_dir.exists()? {
            let ears_parts_manager = PartsManager::new(&ears_dir)?;
            Some(Box::new(ears_parts_manager))
        } else {
            None
        })
    }
}
