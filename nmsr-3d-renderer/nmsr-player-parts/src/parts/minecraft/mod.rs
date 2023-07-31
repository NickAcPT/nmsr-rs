use PlayerBodyPartType::*;

use crate::parts::part::Part;
use crate::parts::provider::{PartsProvider, PlayerPartProviderContext};
use crate::parts::types::PlayerBodyPartType;

pub struct MinecraftPlayerPartsProvider;

macro_rules! body_part {
    // Matcher on many body parts
    ($match_var: ident, $($name: ident {pos: $pos: tt, size: $size: tt, box_uv_start: ($uv_x: expr, $uv_y: expr)}),*) => {
        match $match_var {
            $(
                $name => Some(Part::new_cube(
                    crate::parts::types::PlayerPartTextureType::Skin,
                    $pos,
                    $size,
                    box_uv($uv_x, $uv_y, $size),
                )),
            ),*
            _ => None
        }
    };
}

fn uv_from_pos_and_size(x: u8, y: u8, size_x: u8, size_y: u8) -> [u8; 4] {
    [x, y, x + size_x, y + size_y]
}

fn box_uv(x: u8, y: u8, size: [u8; 3]) -> [[u8; 4]; 6] {
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
        context: PlayerPartProviderContext,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part> {
        let part = body_part!(
            body_part,
            // Base parts
            Body {
                pos: [-4, 12, -2],
                size: [8, 12, 4],
                box_uv_start: (20, 20)
            },
            Head {
                pos: [0, 24, -2],
                size: [8, 8, 8],
                box_uv_start: (8, 8)
            }
        );

        part.into_iter().collect()
    }
}
