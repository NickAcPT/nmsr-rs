use crate::{parts::player_model::PlayerModel, uv::uv_magic::UvImage, uv::Rgba16Image};
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs::DirEntry,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct PartsManager {
    pub all_parts: HashMap<String, UvImage>,
    pub model_parts: HashMap<String, UvImage>,
    pub model_overlays: HashMap<String, UvImage>,
}

impl PartsManager {
    pub fn new(path: &str) -> Result<PartsManager> {
        let root = Path::new(path);

        let mut all_parts = HashMap::<String, UvImage>::with_capacity(8);
        let mut model_parts = HashMap::<String, UvImage>::with_capacity(8);
        let mut model_overlays = HashMap::<String, UvImage>::with_capacity(8);

        Self::load_as_parts(root, &mut all_parts, "")?;
        Self::load_model_specific_parts(root, &mut model_parts)?;

        let overlays_root = root.join("overlays");
        let overlays_root_path = overlays_root.as_path();

        Self::load_as_parts(overlays_root_path, &mut model_overlays, "")?;
        Self::load_model_specific_parts(overlays_root_path, &mut model_overlays)?;

        Ok(PartsManager {
            all_parts,
            model_parts,
            model_overlays,
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
        let directory = dir
            .read_dir()
            .with_context(|| format!("Failed to read directory {:?}", dir))?;

        let loaded_parts: Vec<_> = directory
            .par_bridge()
            .map(|f| Ok(f?.path()))
            .filter(|e: &Result<PathBuf>| e.as_ref().map(|f| f.is_file()).unwrap_or(false))
            .map(|p| -> Result<(String, Rgba16Image)> {
                let path = p?;
                Ok((
                    path.file_name()
                        .ok_or_else(|| anyhow::anyhow!("Failed to get file name"))?
                        .to_str()
                        .ok_or_else(|| anyhow::anyhow!("Failed to convert to str"))?
                        .to_owned(),
                    image::open(path)?.into_rgba16(),
                ))
            })
            .map(|o| -> Result<UvImage> {
                let (name, image) = o?;
                let name = name
                    .chars()
                    .take_while(|p| !char::is_ascii_digit(p))
                    .collect();

                Ok(UvImage::new(name, image))
            })
            .collect();

        for image in loaded_parts.into_iter().flatten() {
            parts_map.insert(format!("{}{}", path_prefix, &image.name), image);
        }

        Ok(())
    }
}
