mod model;

use std::{
    collections::HashMap,
    fs,
    io::{BufWriter, Cursor},
    path::PathBuf,
    vec::Vec,
};

use anyhow::{anyhow, Context, Ok, Result};
use glam::Vec3;
use itertools::Itertools;
use nmsr_rendering::high_level::{parts::part::Part, types::PlayerPartTextureType};

use crate::{
    blockbench::model::{RawProject, RawProjectTexture},
    generator::ModelGenerationProject,
};

use self::model::{ProjectTextureResolution, RawProjectElement, RawProjectElementFace, RawProjectElementFaces};

pub(crate) fn generate_project(project: ModelGenerationProject, output: PathBuf) -> Result<()> {
    let parts = project.generate_parts();
    let grouped_parts = group_by_texture(parts);
    let (resolution, raw_textures) = convert_to_raw_project_textures(&project, &grouped_parts);

    let elements = convert_to_raw_elements(grouped_parts);

    let project = RawProject::new(resolution, elements, raw_textures);

    let project_json =
        serde_json::to_string(&project).context(anyhow!("Failed to serialize project"))?;

    fs::write(output, project_json).context(anyhow!("Failed to write project to file"))?;

    Ok(())
}

fn convert_to_raw_elements(
    grouped_parts: HashMap<PlayerPartTextureType, Vec<Part>>,
) -> Vec<RawProjectElement> {
    let texture_map = grouped_parts
        .keys()
        .enumerate()
        .map(|(i, k)| (*k, i as u32))
        .collect::<HashMap<_, _>>();
    
    grouped_parts
        .into_iter()
        .flat_map(|(texture, parts)| parts)
        .enumerate()
        .filter_map(|(index, part)| match part {
            Part::Cube {
                position,
                size,
                rotation_matrix,
                face_uvs,
                texture,
            } => {
                let from = position;
                let to = position + size;
                
                let texture_id = texture_map.get(&texture).cloned().unwrap_or_default();
                let faces = RawProjectElementFaces::new(texture_id, face_uvs);
                
                Some(RawProjectElement::new(format!("part-{index}"), false, from, to, Vec3::ZERO, faces))
            },

            Part::Quad {
                position,
                size,
                rotation_matrix,
                face_uv,
                normal,
                texture,
            } => None,
        })
        .collect_vec()
}

fn group_by_texture(parts: Vec<Part>) -> HashMap<PlayerPartTextureType, Vec<Part>> {
    let mut result = HashMap::new();

    for (texture, parts) in &parts
        .into_iter()
        .sorted_by_key(|p| p.get_texture())
        .group_by(|p| p.get_texture())
    {
        result.insert(texture, parts.collect());
    }

    result
}

fn convert_to_raw_project_textures(
    project: &ModelGenerationProject,
    grouped_parts: &HashMap<PlayerPartTextureType, Vec<Part>>,
) -> (ProjectTextureResolution, Vec<RawProjectTexture>) {
    let mut resolution = [0, 0];

    let textures = grouped_parts
        .keys()
        .enumerate()
        .map(|(i, k)| {
            let image = if let Some(texture_bytes) = project.get_texture(*k) {
                image::load_from_memory(texture_bytes)
                    .context(anyhow!("Failed to load texture from bytes"))
                    .unwrap()
                    .to_rgba8()
            } else {
                let (w, h) = k.get_texture_size();
                image::RgbaImage::new(w, h)
            };

            let (w, h) = image.dimensions();
            if w > resolution[0] {
                resolution[0] = w;
            }

            if h > resolution[1] {
                resolution[1] = h;
            }

            let image_bytes = {
                let mut bytes = Cursor::new(vec![]);

                {
                    let mut writer = BufWriter::new(&mut bytes);
                    image
                        .write_to(&mut writer, image::ImageOutputFormat::Png)
                        .context(anyhow!("Failed to write empty image to buffer"))
                        .unwrap();
                }

                Ok(bytes.into_inner())
            }
            .context(anyhow!("Failed to write empty image to buffer"))
            .unwrap();

            RawProjectTexture::new(get_texture_name(*k), i as u32, &image_bytes)
        })
        .collect_vec();

    (
        ProjectTextureResolution::new(resolution[0], resolution[1]),
        textures,
    )
}

fn get_texture_name(texture: PlayerPartTextureType) -> String {
    format!(
        "{}.png",
        match texture {
            PlayerPartTextureType::Custom { key, .. } => key.to_string(),
            _ => texture.to_string(),
        }
    )
}
