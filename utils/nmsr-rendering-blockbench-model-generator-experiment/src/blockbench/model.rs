use base64::{engine::general_purpose::STANDARD, Engine};
use glam::Vec3;
use nmsr_rendering::high_level::parts::uv::{CubeFaceUvs, FaceUv};
use serde::Serialize;
use uuid::Uuid;
use xxhash_rust::xxh3::xxh3_128;

use crate::generator::ModelGenerationProject;

#[derive(Debug, Copy, Clone, Serialize)]
pub struct ProjectMeta {
    format_version: &'static str,
    model_format: &'static str,
    box_uv: bool,
}

impl Default for ProjectMeta {
    fn default() -> Self {
        Self {
            format_version: "4.5",
            model_format: "free",
            box_uv: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RawProjectElement {
    name: String,
    box_uv: bool,
    #[serde(rename = "type")]
    r#type: &'static str,
    uuid: Uuid,
    from: Vec3,
    to: Vec3,
    origin: Vec3,
    rotation: Vec3,
    faces: RawProjectElementFaces,
}

impl RawProjectElement {
    pub fn new(
        name: String,
        box_uv: bool,
        from: Vec3,
        to: Vec3,
        origin: Vec3,
        rotation: Vec3,
        faces: RawProjectElementFaces,
    ) -> Self {
        Self {
            uuid: str_to_uuid(&name),
            name,
            box_uv,
            r#type: "cube",
            from,
            to,
            origin,
            rotation,
            faces,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RawProjectElementFace {
    texture: Option<u32>,
    rotation: u32,
    uv: [f32; 4],
}

impl RawProjectElementFace {
    pub fn new(texture: Option<u32>, mut uv: FaceUv, horizontal: bool) -> Self {
        let original_uv = uv;
        let mut rotation = (uv.cw_rotation_count as u32) * 90;

        if original_uv.cw_rotation_count > 0 {
            let current_rotation = 4 - uv.cw_rotation_count;
            if !original_uv.flipped_vertically {
                rotation = (((uv.cw_rotation_count as u32) + 2) % 4) * 90;
            }
            for _ in 0..current_rotation {
                uv = uv.rotate_cw();
            }
        }

        if horizontal {
            uv = uv.flip_horizontally();
        }

        if original_uv.flipped_vertically {
            dbg!(original_uv);
            uv = if original_uv.cw_rotation_count > 0 {
                uv.flip_horizontally()
            } else {
                uv.flip_vertically()
            }
        }

        //if original_uv.flipped_horizontally {
        //    uv = if original_uv.cw_rotation_count > 0 {
        //        uv.flip_vertically()
        //    } else {
        //        uv.flip_horizontally()
        //    }
        //}

        let uv = [
            uv.top_left.x as f32,
            uv.top_left.y as f32,
            uv.bottom_right.x as f32,
            uv.bottom_right.y as f32,
        ];

        Self {
            texture,
            uv,
            rotation,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RawProjectElementFaces {
    north: RawProjectElementFace,
    south: RawProjectElementFace,
    east: RawProjectElementFace,
    west: RawProjectElementFace,
    up: RawProjectElementFace,
    down: RawProjectElementFace,
}

impl Default for RawProjectElementFaces {
    fn default() -> Self {
        let discard = RawProjectElementFace {
            texture: None,
            rotation: 0,
            uv: [0.0, 0.0, 0.0, 0.0],
        };
        
        Self {
            north: discard,
            south: discard,
            east: discard,
            west: discard,
            up: discard,
            down: discard,
        }
    }
}

impl RawProjectElementFaces {
    pub fn new(texture: u32, faces: CubeFaceUvs) -> Self {
        let mut result = Self::default();

        if faces.north != ModelGenerationProject::DISCARD_FACE {
            result.north = RawProjectElementFace::new(Some(texture), faces.north, false);
        }

        if faces.south != ModelGenerationProject::DISCARD_FACE {
            result.south = RawProjectElementFace::new(Some(texture), faces.south, false);
        }

        if faces.east != ModelGenerationProject::DISCARD_FACE {
            result.east = RawProjectElementFace::new(Some(texture), faces.east, false);
        }

        if faces.west != ModelGenerationProject::DISCARD_FACE {
            result.west = RawProjectElementFace::new(Some(texture), faces.west, false);
        }

        if faces.up != ModelGenerationProject::DISCARD_FACE {
            result.up = RawProjectElementFace::new(Some(texture), faces.up, true);
        }

        if faces.down != ModelGenerationProject::DISCARD_FACE {
            result.down = RawProjectElementFace::new(Some(texture), faces.down, true);
        }

        result
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RawProject {
    meta: ProjectMeta,
    resolution: ProjectTextureResolution,
    elements: Vec<RawProjectElement>,
    textures: Vec<RawProjectTexture>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectTextureResolution {
    width: u32,
    height: u32,
}

impl ProjectTextureResolution {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl RawProject {
    pub fn new(
        resolution: ProjectTextureResolution,
        elements: Vec<RawProjectElement>,
        textures: Vec<RawProjectTexture>,
    ) -> Self {
        Self {
            meta: ProjectMeta::default(),
            elements,
            textures,
            resolution,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RawProjectTexture {
    name: String,
    id: u32,
    path: &'static str,
    mode: &'static str,
    visible: bool,
    saved: bool,
    uuid: Uuid,
    source: String,
}

impl RawProjectTexture {
    pub fn new(name: String, id: u32, source: &[u8]) -> Self {
        Self {
            path: "",
            uuid: str_to_uuid(&name),
            name,
            id,
            mode: "bitmap",
            visible: true,
            saved: false,
            source: format!("data:image/png;base64,{}", STANDARD.encode(source)),
        }
    }
}

pub(crate) fn str_to_uuid(s: &str) -> Uuid {
    let mut bytes = xxh3_128(s.as_bytes()).to_be_bytes();
    // Set the version to 4 (random)
    bytes[6] = (bytes[6] & 0x0f) | 0x40;

    Uuid::from_bytes(bytes)
}
