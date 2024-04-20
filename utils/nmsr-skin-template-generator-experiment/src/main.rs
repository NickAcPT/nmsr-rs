use std::{collections::HashMap, iter::repeat};

use anyhow::Ok;

#[cfg(feature = "ears")]
use ears_rs::{
    features::{
        data::{
            ear::{EarAnchor, EarMode},
            snout::SnoutData,
            tail::{TailData, TailMode},
        },
        EarsFeatures,
    },
    parser::EarsFeaturesWriter,
};

use glam::{Vec2, Vec3};
use hsl::HSL;
use image::RgbaImage;
use itertools::Itertools;
use nmsr_player_parts::{
    parts::{
        part::Part,
        provider::{PartsProvider, PlayerPartProviderContext, PlayerPartsProvider},
        uv::FaceUv,
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
    IntoEnumIterator,
};
use rand::Rng;
use strum::EnumIter;

#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq, EnumIter, Default)]
enum FaceOrientation {
    Up,
    Down,
    North,
    
    #[default]
    South,
    East,
    West,
}

impl FaceOrientation {
    pub fn from_normal(normal: Vec3) -> Self {
        // North is -Z / South is +Z
        // East is +X / West is -X
        // Up is +Y / Down is -Y
        let normalized = normal.normalize();
        let (x, y, z) = (normalized.x, normalized.y, normalized.z);

        if y > 0.5 {
            Self::Up
        } else if y < -0.5 {
            Self::Down
        } else if z > 0.5 {
            Self::North
        } else if z < -0.5 {
            Self::South
        } else if x > 0.5 {
            Self::East
        } else if x < -0.5 {
            Self::West
        } else {
            println!("Unknown face orientation: {:?}", normal);
            Self::North
        }
    }
}

#[derive(Debug)]
struct PartTemplateGeneratorContext {
    colors: HashMap<FaceOrientation, HSL>,
}

impl PartTemplateGeneratorContext {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();

        let colors = FaceOrientation::iter()
            .map(|f| {
                (
                    f,
                    hsl::HSL {
                        h: rng.gen_range(0.0..=360.0),
                        s: 0.75,
                        l: 0.7,
                    },
                )
            })
            .collect();

        Self { colors }
    }
}

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "ears")]
    let ears_features = EarsFeatures {
        ear_mode: EarMode::Around,
        ear_anchor: EarAnchor::Center,
        tail: Some(TailData {
            mode: TailMode::Down,
            segments: 1,
            ..Default::default()
        }),
        snout: Some(SnoutData {
            offset: 2,
            width: 4,
            height: 2,
            depth: 2,
        }),
        wing: None,
        claws: true,
        horn: false,
        chest_size: 0f32,
        cape_enabled: false,
        emissive: false,
    };
    
    let context: PlayerPartProviderContext<()> = PlayerPartProviderContext {
        model: nmsr_player_parts::model::PlayerModel::Alex,
        has_hat_layer: true,
        has_layers: true,
        has_cape: false,
        arm_rotation: 0f32,
        shadow_y_pos: None,
        shadow_is_square: false,
        armor_slots: None,
        #[cfg(feature = "ears")]
        ears_features: Some(ears_features),
    };

    #[cfg(feature = "ears")] let parts = {[PlayerPartsProvider::Ears]};
    #[cfg(not(feature = "ears"))] let parts = {[PlayerPartsProvider::Minecraft]};
    let parts = parts
        .iter()
        .flat_map(|provider| {
            PlayerBodyPartType::iter()
                .flat_map(|p| provider.get_parts(&context, p).into_iter().zip(repeat(p)))
        })
        .collect::<Vec<_>>();

    let mut out_textures: HashMap<PlayerPartTextureType, RgbaImage> = HashMap::new();

    let grouped_parts = parts
        .into_iter()
        .sorted_by_key(|(p, _)| p.get_texture())
        .group_by(|(p, _)| p.get_texture());

    for (texture, parts) in grouped_parts.into_iter() {
        let part_texture = out_textures.entry(texture).or_insert_with(|| {
            let (width, height) = texture.get_texture_size();
            RgbaImage::new(width, height)
        });

        for (part, body_part) in parts {
            let part_template_context = PartTemplateGeneratorContext::new();
            
            handle_part_texture(&part_template_context, body_part, part, part_texture);
        }

        if texture == PlayerPartTextureType::Skin {
            ears_rs::utils::strip_alpha(part_texture);

            #[cfg(feature = "ears")]
            ears_rs::parser::v1::writer::EarsWriterV1::write(part_texture, &ears_features)?;
        }
    }

    out_textures.into_iter().for_each(|(texture, img)| {
        img.save(format!("template-{}.png", texture.to_string()))
            .unwrap();
    });

    Ok(())
}

fn handle_part_face(
    part_template_context: &PartTemplateGeneratorContext,
    part: PlayerBodyPartType,
    face: FaceUv,
    orientation: FaceOrientation,
    part_texture: &mut RgbaImage,
) {
    let top_left = Vec2::new(face.top_left.x as f32, face.top_left.y as f32);
    let bottom_right = Vec2::new(face.bottom_right.x as f32, face.bottom_right.y as f32);

    let min = top_left.min(bottom_right);
    let max = top_left.max(bottom_right);


    let Some(mut color) = part_template_context.colors.get(&orientation).copied() else {
        return;
    };
    
    let is_layer = part.is_layer() || part.is_hat_layer() || true;
    
    if is_layer {
        color.s = 1.0;
        color.l = 0.5;
    }
    
    let min_x = min.x as u32;
    let max_x = max.x as u32;
    let min_y = min.y as u32;
    let max_y = max.y as u32;
    for x in min_x..max_x {
        for y in min_y..max_y {
            if is_layer && (x != min_x && x != max_x - 1 && y != min_y && y != max_y - 1) {
                continue;
            }
            
            let (r, g, b) = color.to_rgb();
            let a = if is_layer { 127 } else { 255 };

            let color = image::Rgba([r, g, b, a]);

            part_texture.put_pixel(x, y, color);
        }
    }
}

fn handle_part_texture(
    part_template_context: &PartTemplateGeneratorContext,
    body_part: PlayerBodyPartType,
    part: Part,
    part_texture: &mut RgbaImage,
) {
    match part {
        Part::Cube { face_uvs, .. } => {
            let uvs = [
                face_uvs.north,
                face_uvs.south,
                face_uvs.east,
                face_uvs.west,
                face_uvs.up,
                face_uvs.down,
            ];
            
            let orientations = [
                FaceOrientation::North,
                FaceOrientation::South,
                FaceOrientation::East,
                FaceOrientation::West,
                FaceOrientation::Up,
                FaceOrientation::Down,
            ];

            for (face_uv, orientation) in uvs.iter().zip(orientations.iter()) {
                handle_part_face(
                    part_template_context,
                    body_part,
                    *face_uv,
                    *orientation,
                    part_texture,
                );
            }
        }
        Part::Quad {
            face_uv,
            normal,
            transformation,
            ..
        } => {
            let orientation =
                FaceOrientation::from_normal(transformation.transform_vector3(normal));

            handle_part_face(
                part_template_context,
                body_part,
                face_uv,
                orientation,
                part_texture,
            );
        }
        Part::Group { parts, .. } => {
            for part in parts {
                handle_part_texture(part_template_context, body_part, part, part_texture);
            }
        }
    }
}
