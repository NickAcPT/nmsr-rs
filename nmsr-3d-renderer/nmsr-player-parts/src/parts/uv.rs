use glam::Vec2;

/// Represents a point on a face of a part.
/// The values are in the range 0-255 since Minecraft skin textures are small.
#[derive(Debug, Copy, Clone)]
pub struct FaceUvPoint {
    pub x: UvCoordinate,
    pub y: UvCoordinate,
}

impl FaceUvPoint {
    pub fn to_uv(&self, texture_size: Vec2) -> Vec2 {
        Vec2::new(
            self.x as f32 / texture_size.x,
            self.y as f32 / texture_size.y
        )
    }
}

/// Represents a face of a part.
/// The values are in the range 0-255 since Minecraft skin textures are small.
#[derive(Debug, Copy, Clone)]
pub struct FaceUv {
    pub top_left: FaceUvPoint,
    pub bottom_right: FaceUvPoint,
}

impl FaceUv {
    pub fn flip_vertically(self) -> Self {
        Self {
            top_left: FaceUvPoint { x: self.top_left.x, y: self.bottom_right.y },
            bottom_right: FaceUvPoint { x: self.bottom_right.x, y: self.top_left.y },
        }
    }
    
    pub fn flip_horizontally(self) -> Self {
        Self {
            top_left: FaceUvPoint { x: self.bottom_right.x, y: self.top_left.y },
            bottom_right: FaceUvPoint { x: self.top_left.x, y: self.bottom_right.y },
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CubeFaceUvs {
    pub north: FaceUv,
    pub south: FaceUv,
    pub east: FaceUv,
    pub west: FaceUv,
    pub up: FaceUv,
    pub down: FaceUv,
}

pub fn uv_from_pos_and_size(x: u16, y: u16, size_x: u16, size_y: u16) -> [u16; 4] {
    [x, y, x + size_x, y + size_y]
}

pub fn uv_from_pos_and_size_flipped(x: u16, y: u16, size_x: u16, size_y: u16) -> [u16; 4] {
    [x + size_x, y + size_y, x, y]
}

pub fn box_uv(x: u16, y: u16, size: [u16; 3]) -> [[u16; 4]; 6] {
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

impl From<[[UvCoordinate; 4]; 6]> for CubeFaceUvs {
    fn from(uvs: [[UvCoordinate; 4]; 6]) -> Self {
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

impl From<[UvCoordinate; 4]> for FaceUv {
    fn from(uvs: [UvCoordinate; 4]) -> Self {
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

pub(crate) type UvCoordinate = u16;
