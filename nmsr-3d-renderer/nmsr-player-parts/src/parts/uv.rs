use glam::Vec2;

/// Represents a point on a face of a part.
/// The values are in the range 0-255 since Minecraft skin textures are small.
#[derive(Copy, Clone)]
pub struct FaceUvPoint {
    pub x: u16,
    pub y: u16,
}

impl FaceUvPoint {
    pub fn to_uv(&self, texture_size: Vec2) -> Vec2 {
        Vec2::new(
            self.x as f32 / texture_size.x,
            self.y as f32 / texture_size.y,
        )
    }
}

/// Represents a face of a part.
/// The values are in the range 0-255 since Minecraft skin textures are small.
#[derive(Copy, Clone)]
pub struct FaceUv {
    pub top_left: FaceUvPoint,
    pub bottom_right: FaceUvPoint,
}

#[derive(Copy, Clone)]
pub struct CubeFaceUvs {
    pub north: FaceUv,
    pub south: FaceUv,
    pub east: FaceUv,
    pub west: FaceUv,
    pub up: FaceUv,
    pub down: FaceUv,
}

impl From<[[u16; 4]; 6]> for CubeFaceUvs {
    fn from(uvs: [[u16; 4]; 6]) -> Self {
        Self {
            north: uvs[0].into(),
            south: uvs[1].into(),
            east: uvs[2].into(),
            west: uvs[3].into(),
            up: uvs[4].into(),
            down: uvs[5].into(),
        }
    }
}

impl From<[u16; 4]> for FaceUv {
    fn from(uvs: [u16; 4]) -> Self {
        Self {
            top_left: FaceUvPoint {
                x: uvs[0],
                y: uvs[1],
            },
            bottom_right: FaceUvPoint {
                x: uvs[2],
                y: uvs[3],
            },
        }
    }
}
