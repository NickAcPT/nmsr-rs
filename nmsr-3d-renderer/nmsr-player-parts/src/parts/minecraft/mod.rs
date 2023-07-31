use PlayerBodyPartType::*;

use crate::parts::part::Part;
use crate::parts::provider::{PartsProvider, PlayerPartProviderContext};
use crate::parts::types::PlayerBodyPartType;

pub struct MinecraftPlayerPartsProvider;

macro_rules! body_part {
    // Matcher on many body parts
    ($match_var: expr, $($name: ident {pos: $pos: tt, size: $size: tt, box_uv_start: ($uv_x: expr, $uv_y: expr)}),*) => {
        match $match_var {
            $(
                $name => Some(Part::new_cube(
                    crate::parts::types::PlayerPartTextureType::Skin,
                    $pos,
                    $size,
                    box_uv($uv_x, $uv_y, $size),
                )),
            )*
            _ => None
        }
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
        let non_layer_body_part_type = body_part.get_non_layer_part();

        let part = body_part!(
            non_layer_body_part_type,
            // Base parts
            /*Body {
                pos: [-4, 12, -2],
                size: [8, 12, 4],
                box_uv_start: (20, 20)
            },*/
            Head {
                pos: [-4, 24, -4],
                size: [8, 8, 8],
                box_uv_start: (8, 8)
            },
            LeftLeg {
                pos: [-4, 0, -2],
                size: [4, 12, 4],
                box_uv_start: (20, 52)
            },
            RightLeg {
                pos: [0, 0, -2],
                size: [4, 12, 4],
                box_uv_start: (4, 20)
            }
        );

        if let Some(part) = part {
            if body_part.is_layer() {
                let mut new_part = part.expand(if non_layer_body_part_type == Head {
                    0.5
                } else {
                    0.25
                });

                let box_uv_offset: (i32, i32) = match non_layer_body_part_type {
                    Head => (32, 0),
                    Body => (0, 16),
                    LeftArm => (16, 0),
                    RightArm => (0, 16),
                    LeftLeg => (-16, 0),
                    RightLeg => (0, 16),
                    _ => unreachable!(),
                };

                match new_part {
                    Part::Cube {
                        ref mut face_uvs, ..
                    } => {
                        let current_box_uv = face_uvs.north.top_left;

                        let size = part.get_size();
                        *face_uvs = box_uv(
                            (current_box_uv.x as i32 + box_uv_offset.0) as u16,
                            (current_box_uv.y as i32 + box_uv_offset.1) as u16,
                            [size.x as u16, size.y as u16, size.z as u16],
                        )
                        .into()
                    }
                    Part::Quad {
                        ref mut face_uv, ..
                    } => {
                        todo!("Quad support")
                    }
                }

                return vec![new_part];
            }
        }

        part.into_iter().collect()
    }
}
