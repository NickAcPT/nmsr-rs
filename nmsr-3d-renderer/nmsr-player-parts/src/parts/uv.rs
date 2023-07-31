use glam::Vec2;

/// Represents a point on a face of a part.
/// The values are in the range 0-255 since Minecraft skin textures are small.
pub struct FaceUvPoint {
    pub x: u8,
    pub y: u8,
}

impl FaceUvPoint {
    fn to_uv(&self, texture_size: Vec2) -> Vec2 {
        Vec2::new(
            self.x as f32 / texture_size.x,
            self.y as f32 / texture_size.y,
        )
    }
}

/// Represents a face of a part.
/// The values are in the range 0-255 since Minecraft skin textures are small.
pub struct FaceUv {
    pub top_left: FaceUvPoint,
    pub bottom_right: FaceUvPoint,
}

pub struct CubeFaceUvs {
    pub north: FaceUv,
    pub south: FaceUv,
    pub east: FaceUv,
    pub west: FaceUv,
    pub up: FaceUv,
    pub down: FaceUv
}

impl From<[u8; 4]> for FaceUv {
    fn from(uvs: [u8; 4]) -> Self {
        Self {
            top_left: FaceUvPoint { x: uvs[0], y: uvs[1] },
            bottom_right: FaceUvPoint { x: uvs[2], y: uvs[3] },
        }
    }
}
