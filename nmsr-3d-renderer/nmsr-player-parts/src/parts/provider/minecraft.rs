use std::ops::DerefMut;

use glam::Vec3;

use crate::parts::part::{Part, PartAnchorInfo};
use crate::parts::provider::{PartsProvider, PlayerPartProviderContext};
use crate::types::PlayerBodyPartType;
use crate::types::PlayerBodyPartType::*;

pub struct MinecraftPlayerPartsProvider;

macro_rules! body_part {
    // Matcher on many body parts
    {pos: $pos: tt, size: $size: tt, box_uv_start: ($uv_x: expr, $uv_y: expr)} => {
        body_part! {
            pos: $pos,
            size: $size,
            box_uv_start: ($uv_x, $uv_y),
            texture_type: Skin
        }
    };
    {pos: $pos: tt, size: $size: tt, box_uv_start: ($uv_x: expr, $uv_y: expr), texture_type: $texture_type: ident} => {
        Part::new_cube(
            crate::types::PlayerPartTextureType::$texture_type,
            $pos,
            $size,
            box_uv($uv_x, $uv_y, $size),
        )
    };
}

fn uv_from_pos_and_size(x: u16, y: u16, size_x: u16, size_y: u16) -> [u16; 4] {
    [x, y, x + size_x, y + size_y]
}

fn box_uv(x: u16, y: u16, size: [u16; 3]) -> [[u16; 4]; 6] {
    let size_x = size[0];
    let size_y = size[1];
    let size_z = size[2];

    // Generate UVs for a box with the given size, starting at the given position.
    let north = uv_from_pos_and_size(x, y, size_x, size_y);
    let south = uv_from_pos_and_size(x + size_x + size_z, y, size_x, size_y);
    let east = uv_from_pos_and_size(x - size_z, y, size_z, size_y);
    let west = uv_from_pos_and_size(x + size_x, y, size_z, size_y);
    let up = uv_from_pos_and_size(x, y - size_z, size_x, size_z);
    let down = uv_from_pos_and_size(x + size_x, y - size_z, size_x, size_z);

    // Return the UVs in the order [north, south, east, west, up, down]
    [north, south, east, west, up, down]
}

impl PartsProvider for MinecraftPlayerPartsProvider {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part> {
        if body_part.is_layer() && !context.has_layers {
            return vec![];
        }
        
        let non_layer_body_part_type = body_part.get_non_layer_part();

        let mut part = compute_base_part(non_layer_body_part_type, context);

        perform_arm_part_rotation(non_layer_body_part_type, &mut part, context.arm_rotation);

        if body_part.is_layer() {
            return vec![expand_player_body_part(non_layer_body_part_type, part)];
        }
        
        let mut result = vec![part];

        if body_part == Body && context.has_cape {
            append_cape_part(&mut result);
        }
        
        result
    }
}

fn compute_base_part(non_layer_body_part_type: PlayerBodyPartType, context: &PlayerPartProviderContext) -> Part {
    match non_layer_body_part_type {
        Head => body_part! {
            pos: [-4, 24, -4],
            size: [8, 8, 8],
            box_uv_start: (8, 8)
        },
        Body => body_part! {
            pos: [-4, 12, -2],
            size: [8, 12, 4],
            box_uv_start: (20, 20)
        },
        LeftArm => {
            if context.model.is_slim_arms() {
                body_part! {
                    pos: [-7, 12, -2],
                    size: [3, 12, 4],
                    box_uv_start: (36, 52)
                }
            } else {
                body_part! {
                    pos: [-8, 12, -2],
                    size: [4, 12, 4],
                    box_uv_start: (36, 52)
                }
            }
        }
        RightArm => {
            if context.model.is_slim_arms() {
                body_part! {
                    pos: [4, 12, -2],
                    size: [3, 12, 4],
                    box_uv_start: (44, 20)
                }
            } else {
                body_part! {
                    pos: [4, 12, -2],
                    size: [4, 12, 4],
                    box_uv_start: (44, 20)
                }
            }
        }
        LeftLeg => body_part! {
            pos: [-4, 0, -2],
            size: [4, 12, 4],
            box_uv_start: (20, 52)
        },
        RightLeg => body_part! {
            pos: [0, 0, -2],
            size: [4, 12, 4],
            box_uv_start: (4, 20)
        },
        _ => unreachable!("Got layer body part type when getting non-layer body part type."),
    }
}

fn perform_arm_part_rotation(non_layer_body_part_type: PlayerBodyPartType, part: &mut Part, rotation_angle: f32) {;
    let normal_part_size = compute_base_part(non_layer_body_part_type, &PlayerPartProviderContext::default()).get_size();
    
    if non_layer_body_part_type == LeftArm {
        let rotation = normal_part_size * Vec3::new(-1.0, 2.0, 0.0);
        part.set_anchor(Some(PartAnchorInfo { anchor: rotation }));
    
        part.rotation_mut().z = -rotation_angle;
    } else if non_layer_body_part_type == RightArm {
        let rotation = normal_part_size * Vec3::new(1.0, 2.0, 0.0);
        part.set_anchor(Some(PartAnchorInfo { anchor: rotation }));
    
        part.rotation_mut().z = rotation_angle;
    }
}

fn append_cape_part(result: &mut Vec<Part>) {
    let mut cape = body_part! {
        pos: [-5, 8, 1],
        size: [10, 16, 1],
        box_uv_start: (1, 1),
        texture_type: Cape
    };
            
    cape.set_anchor(Some(PartAnchorInfo {
        anchor: [0.0, 24.0, 2.0].into()
    }));
        
    cape.set_rotation([5.0, 180.0, 0.0].into());
            
    result.push(cape);
}

fn expand_player_body_part(non_layer_body_part_type: PlayerBodyPartType, part: Part) -> Part {
    let expand_offset = get_layer_expand_offset(non_layer_body_part_type);
    let mut new_part = part.expand(expand_offset);
    let box_uv_offset: (i32, i32) = get_body_part_layer_uv_offset(non_layer_body_part_type);
    if let Part::Quad { .. } = new_part {
        unreachable!("Got quad when expanding body part.")
    } else if let Part::Cube {
        ref mut face_uvs, ..
    } = new_part
    {
        let current_box_uv = face_uvs.north.top_left;

        let size = part.get_size();
        *face_uvs = box_uv(
            (current_box_uv.x as i32 + box_uv_offset.0) as u16,
            (current_box_uv.y as i32 + box_uv_offset.1) as u16,
            [size.x as u16, size.y as u16, size.z as u16],
        )
        .into()
    }
    return new_part;
}

fn get_body_part_layer_uv_offset(non_layer_body_part_type: PlayerBodyPartType) -> (i32, i32) {
    match non_layer_body_part_type {
        Head => (32, 0),
        Body => (0, 16),
        LeftArm => (16, 0),
        RightArm => (0, 16),
        LeftLeg => (-16, 0),
        RightLeg => (0, 16),
        _ => unreachable!(
            "Tried to compute UV offset for unknown part {:?}",
            non_layer_body_part_type
        ),
    }
}

fn get_layer_expand_offset(body_part: PlayerBodyPartType) -> f32 {
    match body_part {
        Head => 0.5,
        _ => 0.25,
    }
}
