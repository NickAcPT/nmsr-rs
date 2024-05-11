use std::simd::prelude::{f32x4, u32x4, SimdFloat, SimdPartialOrd, SimdUint};

use arrayvec::ArrayVec;
use glam::{Vec2, Vec3A, Vec4};
use image::Pixel;
use nmsr_rendering::low_level::primitives::{part_primitive::PartPrimitive, vertex::Vertex};

use crate::{
    model::RenderEntry,
    shader::{fragment_shader, vertex_shader, ShaderState, VertexInput, VertexOutput},
};

impl RenderEntry {
    pub fn draw_primitives(&mut self, state: &ShaderState) {
        let vertices = state
            .primitive
            .get_vertices()
            .into_iter()
            .map(|v| apply_vertex_shader(v, state))
            .collect::<Vec<_>>();

        let indices = state.primitive.get_indices();

        let mut grouped_vertices = indices
            .chunks_exact(3)
            .flat_map(|chunk| {
                chunk
                    .iter()
                    .copied()
                    .collect::<ArrayVec<u16, 3>>()
                    .into_inner()
            })
            .collect::<Vec<_>>();

        // Depth sort the triangles
        grouped_vertices.sort_by(|a, b| {
            // Average the z-coordinates of the vertices
            let a = (vertices[a[0] as usize].position.z
                + vertices[a[1] as usize].position.z
                + vertices[a[2] as usize].position.z)
                / 3.0;

            let b = (vertices[b[0] as usize].position.z
                + vertices[b[1] as usize].position.z
                + vertices[b[2] as usize].position.z)
                / 3.0;

            b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
        });

        for triangle_indices in grouped_vertices {
            draw_triangle(self, &triangle_indices, &vertices, state);
        }
    }
}

pub fn draw_triangle(
    entry: &mut RenderEntry,
    indices: &[u16; 3],
    vertices: &[VertexOutput],
    state: &ShaderState,
) {
    // Our triangles are defined by three indices (clockwise)
    let [mut vc, mut va, mut vb] = [
        vertices[indices[0] as usize],
        vertices[indices[1] as usize],
        vertices[indices[2] as usize],
    ];

    va.position.y *= -1.0;
    vb.position.y *= -1.0;
    vc.position.y *= -1.0;

    // Vertices are in NDC space (Our Y axis is flipped, so the top left corner is (-1, 1) and the bottom right corner is (1, -1))

    // Next, we need to rasterize the triangle
    // We'll do this by finding the bounding box of the triangle
    // and then iterating over all pixels in that box

    // Find the bounding box (in screen space)
    let vx = Vec3A::new(va.position.x, vb.position.x, vc.position.x);
    let vy = Vec3A::new(va.position.y, vb.position.y, vc.position.y);

    let (min_x, max_x) = (vx.min_element(), vx.max_element());
    let (min_y, max_y) = (vy.min_element(), vy.max_element());

    // Convert the bounding box to actual screen coordinates
    let min_screen_x: u32 = map_float_u32(min_x, -1.0, 1.0, 0u32, entry.size.width - 1);
    let max_screen_x: u32 = map_float_u32(max_x, -1.0, 1.0, 0u32, entry.size.width - 1);

    let min_screen_y: u32 = map_float_u32(min_y, -1.0, 1.0, 0u32, entry.size.height - 1);
    let max_screen_y: u32 = map_float_u32(max_y, -1.0, 1.0, 0u32, entry.size.height - 1);

    let bbox_width = max_screen_x - min_screen_x;
    let bbox_height = max_screen_y - min_screen_y;

    // Skip out-of-bounds boxes
    if bbox_width == 0 || bbox_height == 0 {
        return;
    }

    let barycentric_state = barycentric_coordinates_state(
        Vec3A::from(va.position),
        Vec3A::from(vb.position),
        Vec3A::from(vc.position),
    );

    let depth_buffer = &mut entry.textures.depth_buffer;
    let depth_width = depth_buffer.width() as usize;

    let color_buffer = &mut entry.textures.output;

    // Iterate over all pixels in the bounding box
    for screen_y in min_screen_y..=max_screen_y {
        let y = map_u32_float(screen_y, 0, entry.size.height, -1.0, 1.0);
        let depth_index = screen_y as usize * depth_width;
        let depth_buf_row = depth_buffer
            .get_mut(depth_index..(depth_index + depth_width))
            .unwrap();

        for base_screen_x in (min_screen_x..=max_screen_x).step_by(4) {
            let screen_x = u32x4::from_array([
                base_screen_x + 0,
                base_screen_x + 1,
                base_screen_x + 2,
                base_screen_x + 3,
            ]);

            // Convert the pixel coordinates to screen space
            let x = map_u32x4_f32x4(
                screen_x,
                u32x4::splat(0),
                u32x4::splat(entry.size.width),
                f32x4::splat(-1.0),
                f32x4::splat(1.0),
            );

            //* println! */("{x}, {y} corresponds to ({screen_x}, {screen_y})");
            // Compute the barycentric coordinates of the pixel
            let barycentric = barycentric_coordinates_x4(x, f32x4::splat(y), &barycentric_state);

            // If the pixel is outside the triangle, skip it
            let is_negative = barycentric.u.is_sign_negative()
                | barycentric.v.is_sign_negative()
                | barycentric.w.is_sign_negative();
            if is_negative.all() {
                continue;
            }

            let depth = barycentric.u * f32x4::splat(va.position.z)
                + barycentric.v * f32x4::splat(vb.position.z)
                + barycentric.w * f32x4::splat(vc.position.z);

            let depth_buf_slice = &mut depth_buf_row[base_screen_x as usize..];
            let buffer_depth = f32x4::from_slice(depth_buf_slice);

            // If all pixels are behind the depth buffer, skip them
            let depth_cmp = f32x4::simd_ge(depth, buffer_depth);
            
            let skip_mask = is_negative | depth_cmp;
            if skip_mask.all() {
                continue;
            }

            // Compute the interpolated vertex attributes
            let bary_u_va = barycentric.u * f32x4::splat(va.old_w_recip);
            let bary_v_vb = barycentric.v * f32x4::splat(vb.old_w_recip);
            let bary_w_vc = barycentric.w * f32x4::splat(vc.old_w_recip);

            // Compute the interpolated w-coordinate
            let interpolated_recip_w = (bary_u_va + bary_v_vb + bary_w_vc).recip();

            // Compute the perspective-corrected texture coordinates
            let tex_coord_u_x = bary_u_va * f32x4::splat(va.tex_coord.x);
            let tex_coord_u_y = bary_u_va * f32x4::splat(va.tex_coord.y);
            let tex_coord_v_x = bary_v_vb * f32x4::splat(vb.tex_coord.x);
            let tex_coord_v_y = bary_v_vb * f32x4::splat(vb.tex_coord.y);
            let tex_coord_w_x = bary_w_vc * f32x4::splat(vc.tex_coord.x);
            let tex_coord_w_y = bary_w_vc * f32x4::splat(vc.tex_coord.y);

            let tex_coord_x =
                interpolated_recip_w * (tex_coord_u_x + tex_coord_v_x + tex_coord_w_x);
            let tex_coord_y =
                interpolated_recip_w * (tex_coord_u_y + tex_coord_v_y + tex_coord_w_y);

            for i in 0..4 {
                // If the pixel is behind the depth buffer, skip it
                if skip_mask.test(i) {
                    continue;
                }

                let tex_coord = Vec2::new(tex_coord_x[i], tex_coord_y[i]);

                let normal = barycentric.u[i] * va.normal
                    + barycentric.v[i] * vb.normal
                    + barycentric.w[i] * vc.normal;

                // Compute the color of the pixel
                let color = fragment_shader(
                    VertexOutput {
                        position: Vec4::ZERO,
                        tex_coord,
                        normal,
                        old_w_recip: 0.0,
                    },
                    state,
                );

                if color[3] == 0 {
                    // Discarded pixel
                    continue;
                }

                // Write the pixel to the output buffer
                let pixel = &mut color_buffer.get_pixel_mut(screen_x[i], screen_y);
                pixel.blend(&image::Rgba(color));

                // Write the depth to the depth buffer
                depth_buf_slice[i] = depth[i];
            }
        }
    }
}

fn map_float_u32(value: f32, old_min: f32, old_max: f32, new_min: u32, new_max: u32) -> u32 {
    if value < old_min {
        return new_min;
    } else if value > old_max {
        return new_max;
    }

    let value = value.max(old_min).min(old_max);

    ((value - old_min) / (old_max - old_min) * (new_max - new_min) as f32 + new_min as f32) as u32
}

fn map_u32_float(value: u32, old_min: u32, old_max: u32, new_min: f32, new_max: f32) -> f32 {
    let value = value.max(old_min).min(old_max);

    (value - old_min) as f32 / (old_max - old_min) as f32 * (new_max - new_min) + new_min
}

fn map_u32x4_f32x4(
    value: u32x4,
    old_min: u32x4,
    old_max: u32x4,
    new_min: f32x4,
    new_max: f32x4,
) -> f32x4 {
    let value = value.max(old_min).min(old_max);

    (value - old_min).cast::<f32>() / (old_max - old_min).cast::<f32>() * (new_max - new_min)
        + new_min
}

fn apply_vertex_shader(vertex: Vertex, state: &ShaderState) -> VertexOutput {
    let mut result = vertex_shader(
        VertexInput {
            position: vertex.position.extend(1.0),
            normal: vertex.normal.into(),
            tex_coord: vertex.uv,
        },
        &state,
    );
    let old_w_recip = result.position.w.recip();

    // Apply perspective divide
    result.position *= old_w_recip;

    result.old_w_recip = old_w_recip;

    result
}

struct BarycentricState {
    v0: Vec2,
    v1: Vec2,
    d00: f32,
    d01: f32,
    d11: f32,
    inv_denom: f32,

    a: Vec2,
}

fn barycentric_coordinates_state(a: Vec3A, b: Vec3A, c: Vec3A) -> BarycentricState {
    let v0 = b.truncate() - a.truncate();
    let v1 = c.truncate() - a.truncate();

    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);

    let denom = (d00 * d11 - d01 * d01).recip();

    BarycentricState {
        v0,
        v1,
        d00,
        d01,
        d11,
        inv_denom: denom,
        a: a.truncate(),
    }
}

struct BaryCoordX4 {
    u: f32x4,
    v: f32x4,
    w: f32x4,
}

#[inline]
fn barycentric_coordinates_x4(x: f32x4, y: f32x4, state: &BarycentricState) -> BaryCoordX4 {
    let v2_x = x - f32x4::splat(state.a.x);
    let v2_y = y - f32x4::splat(state.a.y);

    let d20 = dot_f32x4(
        v2_x,
        v2_y,
        f32x4::splat(state.v0.x),
        f32x4::splat(state.v0.y),
    );
    let d21 = dot_f32x4(
        v2_x,
        v2_y,
        f32x4::splat(state.v1.x),
        f32x4::splat(state.v1.y),
    );

    let v = (f32x4::splat(state.d11) * d20 - f32x4::splat(state.d01) * d21)
        * f32x4::splat(state.inv_denom);
    let w = (f32x4::splat(state.d00) * d21 - f32x4::splat(state.d01) * d20)
        * f32x4::splat(state.inv_denom);
    let u = f32x4::splat(1.0) - v - w;

    BaryCoordX4 { u, v, w }
}

fn dot_f32x4(a_x: f32x4, a_y: f32x4, b_x: f32x4, b_y: f32x4) -> f32x4 {
    return a_x * b_x + a_y * b_y;
}