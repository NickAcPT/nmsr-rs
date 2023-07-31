use crate::parts::part::Part::Cube;
use crate::parts::types::{PlayerBodyPartType, PlayerPartTextureType};
use crate::parts::utils::MinecraftPosition;
use crate::parts::uv::{CubeFaceUvs, FaceUv};

pub struct PartAnchorInfo {
    pub part_type: PlayerBodyPartType,
    pub anchor: MinecraftPosition,
}

pub enum Part {
    /// Represents a cube as a part of a player model.
    Cube {
        position: MinecraftPosition,
        size: MinecraftPosition,
        face_uvs: CubeFaceUvs,
        texture: PlayerPartTextureType,
        anchor: Option<PartAnchorInfo>,
    },
    /// Represents a quad as a part of a player model.
    Quad {
        position: MinecraftPosition,
        size: MinecraftPosition,
        face_uv: FaceUv,
        texture: PlayerPartTextureType,
        anchor: Option<PartAnchorInfo>,
    },
}

impl Part {

    /// Creates a new cube part.
    ///
    /// # Arguments
    ///
    /// * `pos`: The position of the cube. [x, y, z]
    /// * `size`: The size of the cube. [x, y, z]
    /// * `uvs`: The UVs of the cube.
    /// UVs are in the following order: [North, South, East, West, Up, Down]
    /// Each UV is in the following order: [Top left, Bottom right]
    ///
    /// returns: [Part]
    pub fn new_cube(texture: PlayerPartTextureType, pos: [i32; 3], size: [u32; 3], uvs: [[u8; 4]; 6]) -> Self {
        Cube {
            position: MinecraftPosition::new(pos[0] as f32, pos[1] as f32, pos[2] as f32),
            size: MinecraftPosition::new(size[0] as f32, size[1] as f32, size[2] as f32),
            face_uvs: CubeFaceUvs {
                north: uvs[0].into(),
                south: uvs[1].into(),
                east: uvs[2].into(),
                west: uvs[3].into(),
                up: uvs[4].into(),
                down: uvs[5].into(),
            },
            texture,
            anchor: None,
        }
    }
}
