pub mod model;
mod tree;

use std::{
    collections::HashMap,
    fs,
    io::{BufWriter, Cursor},
    path::Path,
    vec::Vec, borrow::Cow,
};

use anyhow::{anyhow, Context, Ok, Result};
use glam::Vec3;
use image::RgbaImage;
use itertools::Itertools;
use nmsr_rendering::high_level::{parts::part::Part, types::PlayerPartTextureType};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    blockbench::model::{str_to_uuid, RawProject, RawProjectTexture},
    generator::ModelGenerationProject,
};

use self::model::{ProjectTextureResolution, RawProjectElement, RawProjectElementFaces};

pub(crate) fn generate_project(project: ModelGenerationProject, output: &Path) -> Result<()> {
    let parts = project.generate_parts();
    let texture_grouped_parts = group_by_texture(parts);
    let outliner_groups = convert_to_outliner(&project, texture_grouped_parts.values().flatten());
    let (resolution, raw_textures) =
        convert_to_raw_project_textures(&project, &texture_grouped_parts);
    let elements = convert_to_raw_elements(&project, texture_grouped_parts);

    let project = RawProject::new(resolution, elements, raw_textures, outliner_groups);

    let project_json =
        serde_json::to_string(&project).context(anyhow!("Failed to serialize project"))?;

    fs::write(output, project_json).context(anyhow!("Failed to write project to file"))?;

    Ok(())
}

fn convert_to_outliner<'a>(
    project: &ModelGenerationProject,
    parts: impl Iterator<Item = &'a Part>,
) -> Vec<serde_json::Value> {
    // Group into a tree structure
    #[derive(Debug)]
    enum Tree {
        Group { name: String, children: Vec<Tree> },
        Part { name: Uuid, group: String },
    }
    impl Tree {
        fn name(&self) -> Cow<'_, str> {
            match self {
                Tree::Group { name, .. } => name.into(),
                Tree::Part { name, .. } => Cow::Owned(name.to_string()),
            }
        }
    }

    fn get_group_name(part: &Part) -> String {
        part.get_group().iter().join("/")
    }

    let root = parts
        .enumerate()
        .sorted_by_key(|p| p.1.get_group())
        .rev()
        .map(|(index, part)| Tree::Part {
            name: str_to_uuid(&project.get_part_name(part.get_name(), index)),
            group: get_group_name(part),
        })
        .inspect(|p| println!("{p:?}"))
        .fold(None, |acc, element| {
            let mut parent_group = acc.unwrap_or_else(|| Tree::Group {
                name: match &element {
                    Tree::Part { group, .. } => group.clone(),
                    _ => unreachable!("We only have parts so far"),
                },
                children: vec![],
            });

            if let Tree::Part { group, .. } = &element {
                if !group.starts_with(&*parent_group.name()) {
                    parent_group = Tree::Group {
                        name: group.clone(),
                        children: vec![parent_group],
                    }
                }
            }

            if let Tree::Group { children, .. } = &mut parent_group {
                children.push(element)
            }

            Some(parent_group)
        });

    let groups = dbg!(root)
        .map(|t| match t {
            Tree::Group { name, children } => children,
            Tree::Part { name, group } => unreachable!("Expected a group, got a part"),
        })
        .unwrap_or(vec![]);

    // Convert to outliner format
    fn to_outliner(element: Tree) -> Value {
        match element {
            Tree::Group { name, children } => json!({
                "name": name,
                "uuid": Uuid::new_v4(),
                "children": children.into_iter().map(to_outliner).collect::<Vec<_>>(),
            }),
            Tree::Part { name, .. } => Value::String(name.to_string()),
        }
    }

    groups.into_iter().map(to_outliner).collect_vec()
}

fn convert_to_raw_elements(
    project: &ModelGenerationProject,
    grouped_parts: HashMap<PlayerPartTextureType, Vec<Part>>,
) -> Vec<RawProjectElement> {
    grouped_parts
        .into_iter()
        .flat_map(|(_, parts)| parts)
        .enumerate()
        .map(|(index, part)| match &part {
            Part::Cube {
                position,
                size,
                last_rotation,
                face_uvs,
                texture,
                name,
                ..
            } => {
                let from = *position;
                let to = *position + *size;

                let faces = RawProjectElementFaces::new(project, *texture, *face_uvs);

                let mut rotation = Vec3::ZERO;
                let mut rotation_anchor = Vec3::ZERO;

                if let Some((rot, anchor)) = *last_rotation {
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

            Part::Quad { name, texture, .. } => {
                let name = name
                    .to_owned()
                    .map_or_else(|| format!("part-{index}"), |s| s.to_string());

                let texture = *texture;

                RawProjectElement::new_quad(name, part, texture, project)
            }
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

pub fn write_png(img: &RgbaImage) -> Result<Vec<u8>> {
    let mut bytes = Cursor::new(vec![]);

    {
        let mut writer = BufWriter::new(&mut bytes);
        img.write_to(&mut writer, image::ImageOutputFormat::Png)
            .context(anyhow!("Failed to write empty image to buffer"))
            .unwrap();
    }

    Ok(bytes.into_inner())
}
