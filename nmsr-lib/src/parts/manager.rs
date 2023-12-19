use image::RgbaImage;

#[cfg(feature = "parallel_iters")]
use rayon::prelude::*;

use tracing::instrument;
use vfs::VfsPath;

use crate::utils::{into_par_iter_if_enabled, open_image_from_vfs};
use crate::{
    errors::{NMSRError, Result},
    parts::speedy_uv::SpeedyUvImage,
};
use crate::{parts::player_model::PlayerModel, uv::uv_magic::UvImage};

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serializable_parts",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    feature = "serializable_parts_rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct PartsManager {
    pub all_parts: Vec<UvImage>,
    pub model_parts: Vec<UvImage>,
    pub model_overlays: Vec<UvImage>,
    pub environment_background: Option<UvImage>,
}

#[derive(Debug)]
pub struct SpeedyUvImagePlayerModel {
    pub alex: SpeedyUvImage,
    pub steve: SpeedyUvImage,

}
#[derive(Debug)]
pub struct SpeedyPartsManager {
    pub no_layers: SpeedyUvImagePlayerModel,
    pub with_layers: SpeedyUvImagePlayerModel,
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
        })
    }

    pub fn new_speedy(root: &VfsPath) -> Result<SpeedyPartsManager> {
        Ok(SpeedyPartsManager {
            no_layers: SpeedyUvImagePlayerModel {
                alex: Self::load_as_parts_speedy(&root.join("Alex")?, "", false)?,
                steve: Self::load_as_parts_speedy(&root.join("Steve")?, "", false)?,
            },

            with_layers: SpeedyUvImagePlayerModel {
                alex: Self::load_as_parts_speedy(&root.join("Alex-Layer")?, "", false)?,
                steve: Self::load_as_parts_speedy(&root.join("Steve-Layer")?, "", false)?,
            },
        })
    }
    
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

            Self::load_as_parts_speedy(&model_parts_dir, dir_name, store_raw_pixels)?;
        }

        Ok(())
    }

    fn load_as_parts_speedy(
        dir: &VfsPath,
        path_prefix: &str,
        store_raw_pixels: bool,
    ) -> Result<SpeedyUvImage> {
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
                .take_while(|p| *p != '.')
                .collect();

            let name = format!("{path_prefix}{name}");

            part_entries.push((name, dir_entry));
        }

        let mut loaded_parts: Vec<_> = into_par_iter_if_enabled!(part_entries)
            .map(|(name, entry)| Ok((name, open_image_from_vfs(&entry)?)))
            .filter_map(|f: Result<(String, RgbaImage)>| f.ok())
            .collect();

        loaded_parts.sort_by(|(a, _), (b, _)| a.cmp(&b));

        let layers = loaded_parts
            .into_iter()
            .map(|(_, image)| image)
            .collect::<Vec<_>>();

        let part = SpeedyUvImage::new(&layers);

        Ok(part)
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
}
