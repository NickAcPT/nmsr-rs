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
        let vertices = self
            .primitive
            .get_vertices()
            .into_iter()
            .map(|v| apply_vertex_shader(v, state))
            .collect::<Vec<_>>();

        let indices = self.primitive.get_indices();

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
        
            a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
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

    let barycentric_state = barycentric_coordinates_state(
        Vec3A::from(va.position),
        Vec3A::from(vb.position),
        Vec3A::from(vc.position),
    );

    // Iterate over all pixels in the bounding box
    for screen_y in min_screen_y..=max_screen_y {
        for screen_x in min_screen_x..=max_screen_x {
            // Convert the pixel coordinates to screen space
            let x = map_u32_float(screen_x, 0, entry.size.width, -1.0, 1.0);
            let y = map_u32_float(screen_y, 0, entry.size.height, -1.0, 1.0);

            //* println! */("{x}, {y} corresponds to ({screen_x}, {screen_y})");
            // Compute the barycentric coordinates of the pixel
            let barycentric = barycentric_coordinates(x, y, &barycentric_state);

            // If the pixel is outside the triangle, skip it
            if barycentric.is_negative_bitmask() != 0 {
                continue;
            }

            let depth = barycentric.x * va.position.z
                + barycentric.y * vb.position.z
                + barycentric.z * vc.position.z;

            // If the pixel is behind the depth buffer, skip it
            if let Some(buffer_depth) = entry
                .textures
                .depth_buffer
                .get_pixel_checked(screen_x, screen_y)
            {
                if std::intrinsics::unlikely(depth >= buffer_depth.0[0]) {
                    continue;
                }
            }

            // Compute the interpolated vertex attributes
            // Compute the interpolated w-coordinate
            let interpolated_recip_w = (barycentric.x * va.old_w_recip
                + barycentric.y * vb.old_w_recip
                + barycentric.z * vc.old_w_recip)
                .recip();
            
            // Compute the perspective-corrected texture coordinates
            let tex_coord = interpolated_recip_w
                * (barycentric.x * va.tex_coord * va.old_w_recip
                    + barycentric.y * vb.tex_coord * vb.old_w_recip
                    + barycentric.z * vc.tex_coord * vc.old_w_recip);

            let normal =
                barycentric.x * va.normal + barycentric.y * vb.normal + barycentric.z * vc.normal;

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
            if let Some(pixel) = entry
                .textures
                .output
                .get_pixel_mut_checked(screen_x, screen_y)
            {
                pixel.blend(&image::Rgba(color));
            }

            // Write the depth to the depth buffer

            if let Some(pixel) = entry
                .textures
                .depth_buffer
                .get_pixel_mut_checked(screen_x, screen_y)
            {
                pixel.0[0] = depth;
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

#[inline]
fn barycentric_coordinates(x: f32, y: f32, state: &BarycentricState) -> Vec3A {
    let v2 = Vec2::new(x, y) - state.a;

    let d20 = v2.dot(state.v0);
    let d21 = v2.dot(state.v1);

    let v = (state.d11 * d20 - state.d01 * d21) * state.inv_denom;
    let w = (state.d00 * d21 - state.d01 * d20) * state.inv_denom;
    let u = 1.0 - v - w;

    Vec3A::new(u, v, w)
}
