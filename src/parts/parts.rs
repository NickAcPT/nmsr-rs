use crate::parts::player_model::PlayerModel;
use crate::UvImage;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::ReadDir;
use std::path::Path;

pub struct PartsManager {
    pub all_parts: HashMap<String, UvImage>,
    pub model_parts: HashMap<String, UvImage>,
}

impl PartsManager {
    pub fn new(path: &str) -> PartsManager {
        let root = Path::new(path);
        let directory = root.read_dir().expect("Path should be readable");

        let mut all_parts = HashMap::<String, UvImage>::new();
        let mut model_parts = HashMap::<String, UvImage>::new();

        Self::load_as_parts(directory, &mut all_parts, "");
        Self::load_model_specific_parts(root, &mut model_parts);

        PartsManager {
            all_parts,
            model_parts,
        }
    }

    fn load_model_specific_parts(root: &Path, mut model_parts: &mut HashMap<String, UvImage>) {
        [PlayerModel::Alex, PlayerModel::Steve]
            .iter()
            .for_each(|model| {
                let dir_name = model.get_dir_name();
                let model_path = root
                    .join(dir_name)
                    .read_dir()
                    .expect("Model path should be readable");

                Self::load_as_parts(model_path, model_parts, dir_name)
            })
    }

    fn load_as_parts(
        directory: ReadDir,
        parts_map: &mut HashMap<String, UvImage>,
        path_prefix: &str,
    ) {
        let loaded_parts: Vec<_> = directory
            .par_bridge()
            .map(|f| f.expect("File Entry to be read"))
            .map(|f| f.path())
            .filter(|e| e.is_file())
            .map(|p| {
                (
                    p.file_name()
                        .expect("File should have a name")
                        .to_str()
                        .expect("File name should be able to be converted")
                        .to_owned(),
                    image::open(p)
                        .expect("Image should load normally")
                        .into_rgba16(),
                )
            })
            .map(|(name, image)| (name, UvImage::new(image)))
            .collect();

        for (name, image) in loaded_parts {
            parts_map.insert(format!("{}{}", path_prefix, name), image);
        }
    }
}
