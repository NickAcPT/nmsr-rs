pub mod model;

use std::collections::HashMap;

use itertools::Itertools;
use nmsr_rendering::high_level::{
    model::ArmorMaterial, parts::part::Part, types::PlayerPartTextureType,
};
use uuid::Uuid;

use crate::{
    blockbench::{
        group_logic::BlockbenchGroupEntry,
        model::{str_to_uuid, RawProject, RawProjectTexture},
    },
    error::Result,
    generator::{ModelGenerationProject, ModelProjectImageIO},
};

use self::model::{ProjectTextureResolution, RawProjectElement};

mod group_logic;

#[cfg(not(feature = "wasm"))]
pub type ProjectOutput = String;

#[cfg(feature = "wasm")]
pub type ProjectOutput = wasm_bindgen::JsValue;

pub type ProjectOutputResult = Result<ProjectOutput>;

pub fn generate_project<M: ArmorMaterial, I: ModelProjectImageIO>(
    mut project: ModelGenerationProject<M, I>,
) -> ProjectOutputResult {
    let parts = project.generate_parts();

    let outliner_groups = generate_outliner_groups(&project, &parts);

    let texture_grouped_parts = group_by_texture(parts);
    project.filter_textures(&texture_grouped_parts.keys().copied().collect_vec());

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

fn generate_outliner_groups<M: ArmorMaterial, I: ModelProjectImageIO>(
    project: &ModelGenerationProject<M, I>,
    parts: &[Part],
) -> serde_json::Value {
    // First, let's store our parts in the following structure:
    let mut root_group = BlockbenchGroupEntry::new_root();

    for (index, part) in parts
        .into_iter()
        .enumerate()
        .sorted_by_key(|(_, p)| p.get_group().len())
    {
        let part_id: Uuid = str_to_uuid(&project.get_part_name(part.get_name(), index));
        // Part groups is a vector of strings, each string being a group name - The last group name is the parent group
        let part_groups: Vec<String> = part.get_group().to_vec();

        // Group our part names in a tree-like structure
        let mut current_group = &mut root_group;
        for group in part_groups {
            // Find our current group in the tree
            current_group = current_group.add_or_get_group(group);
        }
        
        // Add our part to the group
        current_group.add_entry(part_id);
    }

    root_group.to_value()
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
