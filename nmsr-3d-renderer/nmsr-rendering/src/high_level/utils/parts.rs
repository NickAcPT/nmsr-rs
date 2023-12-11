use glam::{Vec2, Vec3};
use itertools::Itertools;
use nmsr_player_parts::parts::{part::Part, uv::FaceUv};

use crate::low_level::primitives::{
    cube::Cube,
    mesh::{Mesh, PrimitiveDispatch},
    quad::Quad,
};

pub fn primitive_convert(part: &Part) -> PrimitiveDispatch {
    let position = Vec3::ZERO;
    let center = position + Vec3::ONE / 2.0;

    let model_transform = part.get_transformation();

    match part {
        Part::Cube { face_uvs, .. } => {
            let texture_size = part.get_texture().get_texture_size();

            Cube::new(
                center,
                Vec3::ONE,
                model_transform,
                uv(&face_uvs.north, texture_size),
                uv(&face_uvs.south, texture_size),
                uv(&face_uvs.up, texture_size),
                uv(&face_uvs.down.flip_horizontally(), texture_size),
                uv(&face_uvs.west, texture_size),
                uv(&face_uvs.east, texture_size),
            )
            .into()
        }
        Part::Quad {
            face_uv,
            texture,
            normal,
            ..
        } => {
            let size = part.get_size();
            
            let x_left = position.x + size.x;
            let x_right = position.x;

            let y_bottom = position.y;
            let y_top = position.y + size.y;

            let z_front = position.z + size.z;
            let z_back = position.z;

            let texture_size = texture.get_texture_size();
            let final_face_uv = uv(face_uv, texture_size);

            Quad::new_with_normal(
                model_transform.transform_point3(Vec3::new(x_right, y_top, z_back)),
                model_transform.transform_point3(Vec3::new(x_left, y_top, z_back)),
                model_transform.transform_point3(Vec3::new(x_right, y_bottom, z_front)),
                model_transform.transform_point3(Vec3::new(x_left, y_bottom, z_front)),
                final_face_uv[0],
                final_face_uv[1],
                final_face_uv[2],
                final_face_uv[3],
                *normal,
            )
            .into()
        }
        Part::Group {
            parts,
            ..
        } => PrimitiveDispatch::Mesh(Mesh::new_with_transform(
            parts.iter().map(primitive_convert).collect_vec(),
            model_transform,
        )),
    }
}

fn uv(face_uvs: &FaceUv, texture_size: (u32, u32)) -> [Vec2; 4] {
    let texture_size = Vec2::new(texture_size.0 as f32, texture_size.1 as f32);

    let mut top_left = face_uvs.top_left.to_uv(texture_size);
    let mut top_right = face_uvs.top_right.to_uv(texture_size);
    let mut bottom_left = face_uvs.bottom_left.to_uv(texture_size);
    let mut bottom_right = face_uvs.bottom_right.to_uv(texture_size);

    let small_offset = 0.000; //Vec2::ONE / texture_size / 32.0;//001;

    top_left += small_offset;
    top_right += Vec2::new(-small_offset, small_offset);
    bottom_right -= small_offset;
    bottom_left += Vec2::new(small_offset, -small_offset);

    [top_left, top_right, bottom_left, bottom_right]
}
