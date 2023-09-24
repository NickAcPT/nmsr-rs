pub mod model;

use std::{
    collections::HashMap,
    io::{BufWriter, Cursor},
    vec::Vec,
};

use anyhow::{anyhow, Context, Ok};
use glam::Vec3;
use image::RgbaImage;
use itertools::Itertools;
use nmsr_rendering::high_level::{parts::part::Part, types::PlayerPartTextureType};

use crate::{
    blockbench::model::{RawProject, RawProjectTexture},
    generator::ModelGenerationProject,
};

use self::model::{ProjectTextureResolution, RawProjectElement, RawProjectElementFaces};

#[cfg(not(feature = "wasm"))]
pub type ProjectOutput = String;

#[cfg(feature = "wasm")]
pub type ProjectOutput = wasm_bindgen::JsValue;

#[cfg(not(feature = "wasm"))]
pub type ProjectOutputResult = anyhow::Result<ProjectOutput>;

#[cfg(feature = "wasm")]
pub type ProjectOutputResult = std::result::Result<ProjectOutput, serde_wasm_bindgen::Error>;

pub fn generate_project(project: ModelGenerationProject) -> ProjectOutputResult {
    let parts = project.generate_parts();
    let texture_grouped_parts = group_by_texture(parts);
    let outliner_groups = vec![];
    let (resolution, raw_textures) =
        convert_to_raw_project_textures(&project, &texture_grouped_parts);
    let elements = convert_to_raw_elements(&project, texture_grouped_parts);

    let project = RawProject::new(resolution, elements, raw_textures, outliner_groups);

    #[cfg(not(feature = "wasm"))]
    {
        let project_json =
            serde_json::to_string(&project).context(anyhow!("Failed to serialize project"))?;

        Ok(project_json)
    }

    #[cfg(feature = "wasm")]
    {
        serde_wasm_bindgen::to_value(&project).map_err(|e| e.into())
    }
}

fn convert_to_raw_elements(
    project: &ModelGenerationProject,
    grouped_parts: HashMap<PlayerPartTextureType, Vec<Part>>,
) -> Vec<RawProjectElement> {
    let parts = grouped_parts
        .into_iter()
        .flat_map(|(_, parts)| parts)
        .enumerate()
        .map(|(index, part)| {
            #[cfg(feature = "markers")]
            let markers = part.part_tracking_data().markers().to_vec();
            
            let name = part.part_tracking_data().name().map(|s| s.as_str());
            let last_rotation = part.part_tracking_data().last_rotation().copied();
            
            let element = match &part {
                Part::Cube {
                    position,
                    size,
                    face_uvs,
                    texture,
                    ..
                } => {
                    let from = *position;
                    let to = *position + *size;

                    let faces = RawProjectElementFaces::new(project, *texture, *face_uvs);

                    let mut rotation = Vec3::ZERO;
                    let mut rotation_anchor = Vec3::ZERO;

                    if let Some((rot, anchor)) = last_rotation {
                        rotation_anchor = anchor.rotation_anchor;
                        rotation = rot;
                    }

                    RawProjectElement::new_cube(
                        project.get_part_name(name.as_deref(), index),
                        false,
                        from,
                        to,
                        rotation_anchor,
                        rotation,
                        faces,
                    )
                }

                Part::Quad { texture, .. } => {
                    let name = name
                        .to_owned()
                        .map_or_else(|| format!("part-{index}"), |s| s.to_string());

                    let texture = *texture;

                    RawProjectElement::new_quad(name, part, texture, project)
                }
            };

            #[cfg(feature = "markers")]
            {
                (markers, element)
            }

            #[cfg(not(feature = "markers"))]
            {
                element
            }
        });

    #[cfg(feature = "markers")]
    let parts = parts.flat_map(|(markers, element)| {
        vec![element].into_iter().chain(
            markers
                .into_iter()
                .map(|m| RawProjectElement::new_null(m.name, m.position)),
        )
    });

    parts.collect_vec()
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
    let textures = grouped_parts
        .keys()
        .sorted_by_key(|&&t| t)
        .enumerate()
        .filter_map(|(i, k)| {
            project
                .get_texture(*k)
                .and_then(|t| write_png(t).ok())
                .map(|t| RawProjectTexture::new(get_texture_name(*k), i as u32, &t))
        })
        .collect_vec();

    let res = project.max_resolution();

    (ProjectTextureResolution::new(res.x, res.y), textures)
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

pub fn write_png(img: &RgbaImage) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Cursor::new(vec![]);

    {
        let mut writer = BufWriter::new(&mut bytes);
        img.write_to(&mut writer, image::ImageOutputFormat::Png)
            .context(anyhow!("Failed to write empty image to buffer"))
            .unwrap();
    }

    Ok(bytes.into_inner())
}
