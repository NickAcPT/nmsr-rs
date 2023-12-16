use std::{collections::HashMap, iter::repeat};

use anyhow::Ok;
use ears_rs::{features::{
    data::{
        ear::{EarAnchor, EarMode},
        snout::SnoutData,
        tail::{TailData, TailMode},
    },
    EarsFeatures,
}, parser::EarsFeaturesWriter};
use glam::Vec2;
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
use rand::seq::SliceRandom;

fn main() -> anyhow::Result<()> {
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
        ears_features: Some(ears_features),
    };

    let parts = [PlayerPartsProvider::Minecraft, PlayerPartsProvider::Ears]
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
            handle_part_texture(body_part, part, part_texture);
        }

        if texture == PlayerPartTextureType::Skin {
            ears_rs::utils::strip_alpha(part_texture);
            
            ears_rs::parser::v1::writer::EarsWriterV1::write(part_texture, &ears_features)?;
        }
    }

    out_textures.into_iter().for_each(|(texture, img)| {
        img.save(format!("template-{}.png", texture.to_string()))
            .unwrap();
    });

    Ok(())
}

const COLORS: [u32; 5] = [0x011627FF, 0x087CA7FF, 0x2EC4B6FF, 0xE71D36FF, 0xFF9F1CFF];

fn handle_part_face(part: PlayerBodyPartType, face: FaceUv, part_texture: &mut RgbaImage) {
    union ColorUnion {
        rgba: u32,
        bytes: [u8; 4],
    }

    let top_left = Vec2::new(face.top_left.x as f32, face.top_left.y as f32);
    let bottom_right = Vec2::new(face.bottom_right.x as f32, face.bottom_right.y as f32);

    let min = top_left.min(bottom_right);
    let max = top_left.max(bottom_right);

    let mut rng = rand::thread_rng();

    let Some(color) = COLORS.choose(&mut rng) else {
        return;
    };

    let min_x = min.x as u32;
    let max_x = max.x as u32;
    let min_y = min.y as u32;
    let max_y = max.y as u32;
    for x in min_x..max_x {
        for y in min_y..max_y {
            if part.is_layer() && (x != min_x && x != max_x - 1 && y != min_y && y != max_y - 1) {
                continue;
            }

            let color = ColorUnion {
                rgba: (*color).reverse_bits(),
            };
            let mut color = image::Rgba(unsafe { color.bytes });

            if part.is_layer() {
                color.0[3] = 0x7F;
            }

            part_texture.put_pixel(x, y, color);
        }
    }
}

fn handle_part_texture(body_part: PlayerBodyPartType, part: Part, part_texture: &mut RgbaImage) {
    match part {
        Part::Cube { face_uvs, .. } => {
            handle_part_face(body_part, face_uvs.up, part_texture);
            handle_part_face(body_part, face_uvs.down, part_texture);
            handle_part_face(body_part, face_uvs.north, part_texture);
            handle_part_face(body_part, face_uvs.south, part_texture);
            handle_part_face(body_part, face_uvs.east, part_texture);
            handle_part_face(body_part, face_uvs.west, part_texture);
        }
        Part::Quad { face_uv, .. } => {
            handle_part_face(body_part, face_uv, part_texture);
        }
        Part::Group { parts, .. } => {
            for part in parts {
                handle_part_texture(body_part, part, part_texture);
            }
        }
    }
}
