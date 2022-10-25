use crate::parts::player_model::PlayerModel;
use crate::uv::uv_magic::UvImage;
use crate::uv::Rgba16Image;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{DirEntry, ReadDir};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PartsManager {
    pub all_parts: HashMap<String, UvImage>,
    pub model_parts: HashMap<String, UvImage>,
}

impl PartsManager {
    pub fn new(path: &str) -> Result<PartsManager> {
        let root = Path::new(path);
        let directory = root
            .read_dir()
            .with_context(|| format!("Failed to read directory {}", path));

        let mut all_parts = HashMap::<String, UvImage>::with_capacity(8);
        let mut model_parts = HashMap::<String, UvImage>::with_capacity(8);

        Self::load_as_parts(directory, &mut all_parts, "")?;
        Self::load_model_specific_parts(root, &mut model_parts)?;

        Ok(PartsManager {
            all_parts,
            model_parts,
        })
    }

    fn load_model_specific_parts(
        root: &Path,
        model_parts: &mut HashMap<String, UvImage>,
    ) -> Result<()> {
        for model in [PlayerModel::Alex, PlayerModel::Steve].iter() {
            let dir_name = model.get_dir_name();
            let model_path = root
                .join(dir_name)
                .read_dir()
                .with_context(|| format!("Failed to read model directory {}", dir_name));

            Self::load_as_parts(model_path, model_parts, dir_name)?;
        }

        Ok(())
    }

    fn load_as_parts(
        directory: Result<ReadDir>,
        parts_map: &mut HashMap<String, UvImage>,
        path_prefix: &str,
    ) -> Result<()> {
        let loaded_parts: Vec<_> = directory?
            .par_bridge()
            .map(|f| Ok(f?))
            .map(|f: Result<DirEntry>| Ok(f?.path()))
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
            .map(|o| -> Result<(String, UvImage)> {
                let (name, image) = o?;
                Ok((name, UvImage::new(image)))
            })
            .collect();

        for (name, image) in loaded_parts.into_iter().flatten() {
            parts_map.insert(format!("{}{}", path_prefix, name), image);
        }

        Ok(())
    }
}
