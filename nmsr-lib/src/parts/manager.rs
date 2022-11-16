use std::collections::HashMap;

#[cfg(feature = "serializable_parts")]
use serde::{Deserialize, Serialize};
use vfs::VfsPath;

use crate::errors::{NMSRError, Result};
use crate::utils::open_image_from_vfs;
use crate::{parts::player_model::PlayerModel, uv::uv_magic::UvImage, uv::Rgba16Image};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serializable_parts", derive(Serialize, Deserialize))]
pub struct PartsManager {
    pub all_parts: HashMap<String, UvImage>,
    pub model_parts: HashMap<String, UvImage>,
    pub model_overlays: HashMap<String, UvImage>,
    pub environment_background: Option<UvImage>,
}

impl PartsManager {
    const ENVIRONMENT_BACKGROUND_NAME: &'static str = "environment_background.png";

    fn is_part_file(path: &VfsPath) -> Result<bool> {
        let name = path.filename();

        Ok(path.is_file()? && name != PartsManager::ENVIRONMENT_BACKGROUND_NAME)
    }

    pub fn new(root: &VfsPath) -> Result<PartsManager> {
        let mut all_parts = HashMap::<String, UvImage>::with_capacity(8);
        let mut model_parts = HashMap::<String, UvImage>::with_capacity(8);
        let mut model_overlays = HashMap::<String, UvImage>::with_capacity(16);

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

    fn load_model_specific_parts(
        root: &VfsPath,
        model_parts: &mut HashMap<String, UvImage>,
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

    fn load_as_parts(
        dir: &VfsPath,
        parts_map: &mut HashMap<String, UvImage>,
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
                .take_while(|p| !char::is_ascii_digit(p) && !char::is_ascii_punctuation(p))
                .collect();

            let name = format!("{}{}", path_prefix, name);

            part_entries.push((name, dir_entry));
        }

        let loaded_parts: Vec<_> = part_entries
            .iter()
            .map(|(name, entry)| Ok((name, open_image_from_vfs(entry)?.into_rgba16())))
            .map(
                |result: Result<(&String, Rgba16Image)>| -> Result<UvImage> {
                    let (name, image) = result?;
                    let uv_image = UvImage::new(name.to_owned(), image, store_raw_pixels);

                    Ok(uv_image)
                },
            )
            .collect();

        for part in loaded_parts {
            let part = part?;
            parts_map.insert(part.name.to_owned(), part);
        }

        Ok(())
    }

    fn load_environment_background(root: &VfsPath) -> Result<Option<UvImage>> {
        let path = &root.join(Self::ENVIRONMENT_BACKGROUND_NAME)?;

        if path.exists()? {
            let image = open_image_from_vfs(path)?;

            Ok(Some(UvImage::new(
                Self::ENVIRONMENT_BACKGROUND_NAME.to_string(),
                image.into_rgba16(),
                true,
            )))
        } else {
            Ok(None)
        }
    }
}
