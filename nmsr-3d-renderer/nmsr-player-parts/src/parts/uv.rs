use glam::Vec2;

/// Represents a point on a face of a part.
/// The values are in the range 0-255 since Minecraft skin textures are small.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FaceUvPoint {
    pub x: UvCoordinate,
    pub y: UvCoordinate,
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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FaceUv {
    pub top_left: FaceUvPoint,
    pub top_right: FaceUvPoint,
    pub bottom_left: FaceUvPoint,
    pub bottom_right: FaceUvPoint,
}

impl FaceUv {
    pub const fn new(
        top_left_x: UvCoordinate,
        top_left_y: UvCoordinate,
        bottom_right_x: UvCoordinate,
        bottom_right_y: UvCoordinate,
    ) -> Self {
        Self {
            top_left: FaceUvPoint {
                x: top_left_x,
                y: top_left_y,
            },
            top_right: FaceUvPoint {
                x: bottom_right_x,
                y: top_left_y,
            },
            bottom_left: FaceUvPoint {
                x: top_left_x,
                y: bottom_right_y,
            },
            bottom_right: FaceUvPoint {
                x: bottom_right_x,
                y: bottom_right_y,
            },
        }
    }

    pub fn flip_vertically(self) -> Self {
        Self {
            top_left: self.bottom_left,
            top_right: self.bottom_right,
            bottom_left: self.top_left,
            bottom_right: self.top_right,
        }
    }

    pub fn flip_horizontally(self) -> Self {
        Self {
            top_left: self.top_right,
            top_right: self.top_left,
            bottom_left: self.bottom_right,
            bottom_right: self.bottom_left,
        }
    }

    pub fn rotate_cw(self) -> Self {
        Self {
            top_left: self.top_right,
            top_right: self.bottom_right,
            bottom_right: self.bottom_left,
            bottom_left: self.top_left,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CubeFaceUvs {
    pub north: FaceUv,
    pub south: FaceUv,
    pub east: FaceUv,
    pub west: FaceUv,
    pub up: FaceUv,
    pub down: FaceUv,
}

pub fn uv_from_pos_and_size(x: u16, y: u16, size_x: u16, size_y: u16) -> FaceUv {
    FaceUv::new(x, y, x + size_x, y + size_y)
}

pub fn box_uv(x: u16, y: u16, size: [u16; 3]) -> CubeFaceUvs {
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
    CubeFaceUvs {
        north,
        south,
        east,
        west,
        up,
        down,
    }
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
        Self::new(uvs[0], uvs[1], uvs[2], uvs[3])
    }
}

impl From<[UvCoordinate; 8]> for FaceUv {
    fn from(uvs: [UvCoordinate; 8]) -> Self {
        Self {
            top_left: FaceUvPoint {
                x: uvs[0],
                y: uvs[1],
            },
            top_right: FaceUvPoint {
                x: uvs[2],
                y: uvs[3],
            },
            bottom_left: FaceUvPoint {
                x: uvs[4],
                y: uvs[5],
            },
            bottom_right: FaceUvPoint {
                x: uvs[6],
                y: uvs[7],
            },
        }
    }
}

pub(crate) type UvCoordinate = u16;
