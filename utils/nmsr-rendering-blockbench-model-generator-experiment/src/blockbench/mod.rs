pub mod model;

use std::{collections::HashMap, vec::Vec};

use itertools::Itertools;
use nmsr_rendering::high_level::{
    model::ArmorMaterial, parts::part::Part, types::PlayerPartTextureType,
};

use crate::{
    blockbench::model::{RawProject, RawProjectTexture},
    error::Result,
    generator::{ModelGenerationProject, ModelProjectImageIO},
};

use self::model::{ProjectTextureResolution, RawProjectElement};

#[cfg(not(feature = "wasm"))]
pub type ProjectOutput = String;

#[cfg(feature = "wasm")]
pub type ProjectOutput = wasm_bindgen::JsValue;

pub type ProjectOutputResult = Result<ProjectOutput>;

pub fn generate_project<M: ArmorMaterial, I: ModelProjectImageIO>(
    mut project: ModelGenerationProject<M, I>,
) -> ProjectOutputResult {
    let parts = project.generate_parts();

    let texture_grouped_parts = group_by_texture(parts);
    project.filter_textures(&texture_grouped_parts.keys().copied().collect_vec());

    let outliner_groups = vec![];
    let (resolution, raw_textures) =
        convert_to_raw_project_textures(&project, &texture_grouped_parts);
    let elements = convert_to_raw_elements(&project, texture_grouped_parts)?;

    let project = RawProject::new(resolution, elements, raw_textures, outliner_groups);

    #[cfg(not(feature = "wasm"))]
    {
        let project_json = serde_json::to_string(&project)?;

        Ok(project_json)
    }

    #[cfg(feature = "wasm")]
    {
        serde_wasm_bindgen::to_value(&project).map_err(|e| e.into())
    }
}

fn convert_to_raw_elements<M: ArmorMaterial, I: ModelProjectImageIO>(
    project: &ModelGenerationProject<M, I>,
    grouped_parts: HashMap<PlayerPartTextureType, Vec<Part>>,
) -> Result<Vec<RawProjectElement>> {
    let parts = grouped_parts
        .into_iter()
        .flat_map(|(_, parts)| parts)
        //.filter(|p| p.get_name().map(|n| n.contains("Tail")).unwrap_or_default())
        .enumerate()
        .map(|(index, part)| -> Result<_> {
            #[cfg(feature = "markers")]
            let markers = part.part_tracking_data().markers().to_vec();

            let name = part.part_tracking_data().name().map(|s| s.as_str());

            let element = match &part {
                part => {
                    let name = project.get_part_name(name, index);

                    RawProjectElement::new_primitive(name, &part, part.get_texture(), project)?
                }
            };

            #[cfg(feature = "markers")]
            {
                Ok((markers, element))
            }

            #[cfg(not(feature = "markers"))]
            {
                Ok(element)
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

    let mut result = Vec::new();

    for part in parts {
        result.push(part?);
    }

    Ok(result)
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

fn convert_to_raw_project_textures<M: ArmorMaterial, I: ModelProjectImageIO>(
    project: &ModelGenerationProject<M, I>,
    grouped_parts: &HashMap<PlayerPartTextureType, Vec<Part>>,
) -> (ProjectTextureResolution, Vec<RawProjectTexture>) {
    let textures = grouped_parts
        .keys()
        .sorted_by_key(|&&t| t)
        .enumerate()
        .filter_map(|(i, k)| {
            project
                .get_texture(*k)
                .and_then(|t| project.image_io().write_png(t).ok())
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
