use base64::{engine::general_purpose::STANDARD, Engine};
use glam::Vec3;
use nmsr_rendering::high_level::parts::uv::{FaceUv, CubeFaceUvs};
use serde::Serialize;
use uuid::Uuid;
use xxhash_rust::xxh3::xxh3_128;

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
    pub fn new(name: String, box_uv: bool, from: Vec3, to: Vec3, origin: Vec3, rotation: Vec3, faces: RawProjectElementFaces) -> Self {
        Self {
            uuid: str_to_uuid(&name),
            name,
            box_uv,
            r#type: "cube",
            from,
            to,
            origin,
            rotation,
            faces
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RawProjectElementFace {
    texture: u32,
    uv: [u16; 4],
}

impl RawProjectElementFace {
    pub fn new(texture: u32, uv: FaceUv) -> Self {
        let uv = [uv.top_left.x, uv.top_left.y, uv.bottom_right.x, uv.bottom_right.y];
        Self { texture, uv }
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

impl RawProjectElementFaces {
    pub fn new(
        texture: u32,
        faces: CubeFaceUvs
    ) -> Self {
        Self {
            north: RawProjectElementFace::new(texture, faces.north),
            south: RawProjectElementFace::new(texture, faces.south),
            east: RawProjectElementFace::new(texture, faces.east),
            west: RawProjectElementFace::new(texture, faces.west),
            up: RawProjectElementFace::new(texture, faces.up),
            down: RawProjectElementFace::new(texture, faces.down),
        }
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
    pub fn new(width: u32, height: u32) -> Self { Self { width, height } }
}

impl RawProject {
    pub fn new(resolution: ProjectTextureResolution, elements: Vec<RawProjectElement>, textures: Vec<RawProjectTexture>) -> Self {
        Self {
            meta: ProjectMeta::default(),
            elements,
            textures,
            resolution
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
