use base64::{engine::general_purpose::STANDARD, Engine};
use glam::{Vec2, Vec3, Affine3A};
use itertools::Itertools;
use nmsr_rendering::{
    high_level::{
        model::ArmorMaterial,
        parts::{uv::{CubeFaceUvs, FaceUv}, part::Part},
        types::PlayerPartTextureType, utils::parts::primitive_convert,
    },
    low_level::primitives::part_primitive::PartPrimitive,
};
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;
use xxhash_rust::xxh3::xxh3_128;

use crate::{
    error::Result,
    generator::{ModelGenerationProject, ModelProjectImageIO},
};

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
        Self(
            json!({
                "uuid": str_to_uuid(&name),
                "name": name,
                "box_uv": box_uv,
                "type": "cube",
                "from": from,
                "to": to,
                "origin": origin,
                "rotation": rotation,
                "faces": faces,
            })
            .into(),
        )
    }

    pub fn new_null(name: String, origin: Vec3) -> Self {
        Self(
            json!({
                "uuid": str_to_uuid(&name),
                "name": name,
                "type": "null_object",
                "position": origin,
            })
            .into(),
        )
    }

    pub fn new_primitive<M: ArmorMaterial, I: ModelProjectImageIO>(
        name: String,
        part: &Part,
        texture: PlayerPartTextureType,
        project: &ModelGenerationProject<M, I>,
    ) -> Result<Self> {
        fn random_names(a: &str, b: &str) -> (String, String) {
            let (a_new, b_new) = Uuid::new_v4().as_u64_pair();

            (format!("{a}{a_new:x}"), format!("{b}{b_new:x}"))
        }
        let converted = primitive_convert(&part);
        
        let affine = part.get_transformation();
        let (_, rotation, translation) = affine.to_scale_rotation_translation();
        
        let rotation_anchor = part.part_tracking_data().last_rotation_origin().unwrap_or(translation);
        let origin = rotation_anchor;
        
        let affine_inv = Affine3A::from_rotation_translation(rotation, origin).inverse();
        
        let (r_x, r_y, r_z) = rotation.to_euler(glam::EulerRot::XYZ);
        
        let origin = json!([
            origin.x,
            origin.y,
            origin.z,
        ]);
        
        let rotation = json!([
            r_x.to_degrees(),
            r_y.to_degrees(),
            r_z.to_degrees(),
        ]);
        
        let texture_id = project.get_texture_id(texture)?;

        let vertices = converted.get_vertices_grouped();
        
        let mut faces = json!({});
        let mut vertices_map = json!({});

        for (t_1, t_2) in vertices.into_iter().tuples() {
            let [vc, va, vd] = t_1;
            let [_, vb, _] = t_2;
            
            let [a, b, c, d] = [
                affine_inv.transform_point3(va.position),
                affine_inv.transform_point3(vb.position),
                affine_inv.transform_point3(vc.position),
                affine_inv.transform_point3(vd.position),
            ];

            let ((va_name, vb_name), (vc_name, vd_name)) = (
                random_names("a", "b"),
                random_names("c", "d"),
            );

            vertices_map[&va_name] = json!([a.x, a.y, a.z]);
            vertices_map[&vb_name] = json!([b.x, b.y, b.z]);
            vertices_map[&vc_name] = json!([c.x, c.y, c.z]);
            vertices_map[&vd_name] = json!([d.x, d.y, d.z]);
            
            let uv_size = texture.get_texture_size();
            let uv_size = Vec2::new(uv_size.0 as f32, uv_size.1 as f32);

            let [va_uv, vb_uv, vc_uv, vd_uv] = [
                project.handle_single_coordinate(texture, va.uv * uv_size),
                project.handle_single_coordinate(texture, vb.uv * uv_size),
                project.handle_single_coordinate(texture, vc.uv * uv_size),
                project.handle_single_coordinate(texture, vd.uv * uv_size),
            ];

            let uv = json!({
                &va_name: [va_uv.x, va_uv.y],
                &vc_name: [vc_uv.x, vc_uv.y],
                &vb_name: [vb_uv.x, vb_uv.y],
                &vd_name: [vd_uv.x, vd_uv.y],
            });

            let face = json!({
                "texture": texture_id,
                "uv": uv,
                "vertices": [
                    &va_name,
                    &vc_name,
                    &vd_name,
                    &vb_name,
                ]
            });

            faces[va_name] = face;
        }

        let render_order = if name.contains("Layer") {
            "in_front"
        } else {
            "behind"
        };
        
        Ok(Self(
            json!({
                "uuid": str_to_uuid(&name),
                "name": name,
                "box_uv": false,
                "type": "mesh",
                "render_order": render_order,
                "origin": origin,
                "rotation": rotation,
                "vertices": vertices_map,
                "faces": faces,
            })
            .into(),
        ))
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RawProjectElementFace {
    texture: Option<u32>,
    uv: [f32; 4],
}

impl RawProjectElementFace {
    pub const UV_OFFSET: f32 = 0.05;

    pub fn new<M: ArmorMaterial, I: ModelProjectImageIO>(
        project: &ModelGenerationProject<M, I>,
        texture: PlayerPartTextureType,
        uv: FaceUv,
    ) -> Result<Self> {
        let uv = project.handle_face(texture, uv);
        let texture_id = project.get_texture_id(texture)?;

        let [top_left_uv, _, bottom_right_uv, _] = shrink_rectangle(
            [
                [uv.top_left.x, uv.top_left.y],
                [uv.top_right.x, uv.top_right.y],
                [uv.bottom_right.x, uv.bottom_right.y],
                [uv.bottom_left.x, uv.bottom_left.y],
            ],
            RawProjectElementFace::UV_OFFSET,
        );

        let uv = [
            top_left_uv[0],
            top_left_uv[1],
            bottom_right_uv[0],
            bottom_right_uv[1],
        ];

        Ok(Self {
            texture: Some(texture_id),
            uv,
        })
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

#[derive(Debug, Clone, Copy)]
pub struct ModelFaceUv {
    pub top_left: Vec2,
    pub top_right: Vec2,
    pub bottom_right: Vec2,
    pub bottom_left: Vec2,
}

impl RawProjectElementFaces {
    pub fn new<M: ArmorMaterial, I: ModelProjectImageIO>(
        project: &ModelGenerationProject<M, I>,
        texture: PlayerPartTextureType,
        faces: CubeFaceUvs,
    ) -> Result<Self> {
        Ok(Self {
            north: RawProjectElementFace::new(project, texture, faces.north)?,
            south: RawProjectElementFace::new(project, texture, faces.south)?,
            east: RawProjectElementFace::new(project, texture, faces.east)?,
            west: RawProjectElementFace::new(project, texture, faces.west)?,
            up: RawProjectElementFace::new(
                project,
                texture,
                faces.up.flip_horizontally().flip_vertically(),
            )?,
            down: RawProjectElementFace::new(project, texture, faces.down.flip_horizontally())?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RawProject {
    meta: ProjectMeta,
    resolution: ProjectTextureResolution,
    elements: Vec<RawProjectElement>,
    textures: Vec<RawProjectTexture>,
    outliner: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectTextureResolution {
    width: f32,
    height: f32,
}

impl ProjectTextureResolution {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl RawProject {
    pub fn new(
        resolution: ProjectTextureResolution,
        elements: Vec<RawProjectElement>,
        textures: Vec<RawProjectTexture>,
        outliner: Value,
    ) -> Self {
        Self {
            meta: ProjectMeta::default(),
            elements,
            textures,
            resolution,
            outliner,
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

pub fn shrink_rectangle(points: [[f32; 2]; 4], factor: f32) -> [[f32; 2]; 4] {
    let center = [
        (points[0][0] + points[1][0] + points[2][0] + points[3][0]) / 4.,
        (points[0][1] + points[1][1] + points[2][1] + points[3][1]) / 4.,
    ];

    fn distance_to(a: [f32; 2], other: [f32; 2]) -> f32 {
        ((a[0] - other[0]).powi(2) + (a[1] - other[1]).powi(2)).sqrt()
    }

    let mut new_points = [[0.0; 2]; 4];
    for (i, point) in points.iter().enumerate() {
        let distance_to_center = distance_to(*point, center);
        let new_distance_to_center = distance_to_center - factor;

        let new_point = [
            center[0] + (point[0] - center[0]) * new_distance_to_center / distance_to_center,
            center[1] + (point[1] - center[1]) * new_distance_to_center / distance_to_center,
        ];

        new_points[i] = new_point;
    }

    new_points
}
