use crate::errors::{NMSRError, Result};
use crate::{parts::player_model::PlayerModel, uv::uv_magic::UvImage, uv::Rgba16Image};
use rayon::prelude::*;
use std::{collections::HashMap, path::Path};

#[derive(Debug)]
pub struct PartsManager {
    pub all_parts: HashMap<String, UvImage>,
    pub model_parts: HashMap<String, UvImage>,
    pub model_overlays: HashMap<String, UvImage>,
    pub environment_background: Option<Rgba16Image>,
}

impl PartsManager {
    const ENVIRONMENT_BACKGROUND_NAME: &'static str = "environment_background";

    fn is_part_file(path: impl AsRef<Path>) -> Result<bool> {
        let path = path.as_ref();
        let name = path
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| NMSRError::InvalidPath(path.to_path_buf()))?;

        Ok(path.is_file() && name != PartsManager::ENVIRONMENT_BACKGROUND_NAME)
    }

    pub fn new(path: &str) -> Result<PartsManager> {
        let root = Path::new(path);

        let mut all_parts = HashMap::<String, UvImage>::with_capacity(8);
        let mut model_parts = HashMap::<String, UvImage>::with_capacity(8);
        let mut model_overlays = HashMap::<String, UvImage>::with_capacity(16);

        Self::load_as_parts(root, &mut all_parts, "")?;
        Self::load_model_specific_parts(root, &mut model_parts)?;

        let overlays_root = root.join("overlays");
        let overlays_root_path = overlays_root.as_path();

        Self::load_as_parts(overlays_root_path, &mut model_overlays, "")?;
        Self::load_model_specific_parts(overlays_root_path, &mut model_overlays)?;

        let environment_background = Self::load_environment_background(root)?;

        Ok(PartsManager {
            all_parts,
            model_parts,
            model_overlays,
            environment_background,
        })
    }

    fn load_model_specific_parts(
        root: &Path,
        model_parts: &mut HashMap<String, UvImage>,
    ) -> Result<()> {
        for model in [PlayerModel::Alex, PlayerModel::Steve].iter() {
            let dir_name = model.get_dir_name();

            Self::load_as_parts(root.join(dir_name).as_path(), model_parts, dir_name)?;
        }

        Ok(())
    }

    fn load_as_parts(
        dir: &Path,
        parts_map: &mut HashMap<String, UvImage>,
        path_prefix: &str,
    ) -> Result<()> {
        let directory = dir.read_dir().map_err(NMSRError::IoError)?;

        let mut part_entries = vec![];

        for dir_entry in directory {
            let entry = dir_entry.map(|e| e.path()).map_err(NMSRError::IoError)?;

            // Skip non part files
            if !Self::is_part_file(&entry)? {
                continue;
            }

            // Compute map entry key
            let name: String = entry
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or_else(|| NMSRError::InvalidPath(entry.to_owned()))?
                .chars()
                .take_while(|p| !char::is_ascii_digit(p) && !char::is_ascii_punctuation(p))
                .collect();

            let name = format!("{}{}", path_prefix, name);

            part_entries.push((name, entry));
        }

        let mut loaded_parts = vec![];

        part_entries
            .par_iter()
            .map(|(name, entry)| {
                let image = image::open(&entry)
                    .map_err(NMSRError::ImageError)?
                    .into_rgba16();

                Ok((name, image))
            })
            .map(
                |result: Result<(&String, Rgba16Image)>| -> Result<UvImage> {
                    let (name, image) = result?;
                    let uv_image = UvImage::new(name.to_owned(), image);

                    Ok(uv_image)
                },
            )
            .collect_into_vec(&mut loaded_parts);

        for part in loaded_parts {
            let part = part?;
            parts_map.insert(part.name.to_owned(), part);
        }

        Ok(())
    }

    fn load_environment_background(root: &Path) -> Result<Option<Rgba16Image>> {
        let path = &root
            .join(Self::ENVIRONMENT_BACKGROUND_NAME)
            .with_extension("png");
        if path.exists() {
            let image = image::open(path).map_err(NMSRError::ImageError)?;
            Ok(Some(image.into_rgba16()))
        } else {
            Ok(None)
        }
    }
}
