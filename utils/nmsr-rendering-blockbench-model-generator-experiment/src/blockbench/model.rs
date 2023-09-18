use base64::{engine::general_purpose::STANDARD, Engine};
use glam::Vec3;
use nmsr_rendering::{
    high_level::{
        parts::{
            part::Part,
            uv::{CubeFaceUvs, FaceUv},
        },
        pipeline::scene::primitive_convert,
    },
    low_level::primitives::mesh::PrimitiveDispatch,
};
use serde::Serialize;
use serde_json::{json, Value};
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
#[repr(transparent)]
pub struct RawProjectElement(Value);

impl RawProjectElement {
    pub fn new_cube(
        name: String,
        box_uv: bool,
        from: Vec3,
        to: Vec3,
        origin: Vec3,
        rotation: Vec3,
        faces: RawProjectElementFaces,
    ) -> Self {
        Self(json!({
            "uuid": str_to_uuid(&name),
            "name": name,
            "box_uv": box_uv,
            "type": "cube",
            "from": from,
            "to": to,
            "origin": origin,
            "rotation": rotation,
            "faces": faces,
        }))
    }

    pub fn new_quad(name: String, mut part: Part, texture_size: (f32, f32), texture_id: u32) -> Self {
        fn random_names(a: &str, b: &str) -> (String, String) {
            let (a_new,b_new) = Uuid::new_v4().as_u64_pair();
            
            (format!("{a}{a_new:x}"), format!("{b}{b_new:x}"))
        }
        
        let uv = part.get_face_uv();
        
        let converted = primitive_convert(&part);

        let (top_left, top_right) = random_names("top_left", "top_right");
        let (bottom_left, bottom_right) = random_names("bottom_left", "bottom_right");
        
        let (uv_width, uv_height) = texture_size;
        
        dbg!(&part);
        
        let result = if let PrimitiveDispatch::Quad(quad) = converted {
            json!({
                "uuid": str_to_uuid(&name),
                "name": name,
                "box_uv": false,
                "type": "mesh",
                "origin": Vec3::ZERO,
                "rotation": Vec3::ZERO,
                "vertices": {
                    &top_left: [
                        quad.top_left.position.x,
                        quad.top_left.position.y,
                        quad.top_left.position.z,
                    ],
                    &top_right: [
                        quad.top_right.position.x,
                        quad.top_right.position.y,
                        quad.top_right.position.z,
                    ],
                    &bottom_right: [
                        quad.bottom_right.position.x,
                        quad.bottom_right.position.y,
                        quad.bottom_right.position.z,
                    ],
                    &bottom_left: [
                        quad.bottom_left.position.x,
                        quad.bottom_left.position.y,
                        quad.bottom_left.position.z,
                    ],
                },
                "faces": {
                    "face": {
                        "texture": texture_id,
                        "uv": {
                            &top_left: [
                                quad.top_left.uv.x * uv_width,
                                quad.top_left.uv.y * uv_height,
                            ],
                            &top_right: [
                                quad.top_right.uv.x * uv_width,
                                quad.top_right.uv.y * uv_height,
                            ],
                            &bottom_right: [
                                quad.bottom_right.uv.x * uv_width,
                                quad.bottom_right.uv.y * uv_height,
                            ],
                            &bottom_left: [
                                quad.bottom_left.uv.x * uv_width,
                                quad.bottom_left.uv.y * uv_height,
                            ],
                        },
                        "vertices": [
                            &top_left,
                            &top_right,
                            &bottom_right,
                            &bottom_left,
                        ]
                    }
                },
            })
        } else {
            unreachable!("Expected a quad primitive, got something else")
        };

        Self(result)
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

        if original_uv.flipped_horizontally {
            uv = if original_uv.cw_rotation_count > 0 {
                uv.flip_vertically()
            } else {
                uv.flip_horizontally()
            }
        }

        let offset = 0.032;

        let uv = [
            uv.top_left.x as f32 + offset,
            uv.top_left.y as f32 + offset,
            uv.bottom_right.x as f32 - offset,
            uv.bottom_right.y as f32 - offset,
        ];

        Self {
            texture,
            uv,
            rotation,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
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
        let RawProjectElementFaces {
            ref mut north,
            ref mut south,
            ref mut east,
            ref mut west,
            ref mut up,
            ref mut down,
        } = result;

        if faces.north != ModelGenerationProject::DISCARD_FACE {
            *north = RawProjectElementFace::new(Some(texture), faces.north, false);
        }

        if faces.south != ModelGenerationProject::DISCARD_FACE {
            *south = RawProjectElementFace::new(Some(texture), faces.south, false);
        }

        if faces.east != ModelGenerationProject::DISCARD_FACE {
            *east = RawProjectElementFace::new(Some(texture), faces.east, false);
        }

        if faces.west != ModelGenerationProject::DISCARD_FACE {
            *west = RawProjectElementFace::new(Some(texture), faces.west, false);
        }

        if faces.up != ModelGenerationProject::DISCARD_FACE {
            *up = RawProjectElementFace::new(Some(texture), faces.up, true);
        }

        if faces.down != ModelGenerationProject::DISCARD_FACE {
            *down = RawProjectElementFace::new(Some(texture), faces.down, true);
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
