#![allow(dead_code)]

// Re-export the Vec3 type from the glam crate
pub use glam::Vec3;
pub use glam::Vec2;

pub mod primitives;
pub mod quad;
pub mod cube;


pub fn generate_matrix(camera: Vec3, aspect_ratio: f32) -> glam::Mat4 {
    let projection = glam::Mat4::perspective_rh(45f32.to_radians(), aspect_ratio, 1.0, 100.0);

    let camera = Vec3::new(camera.x, camera.y, camera.z);

    // Z- is front, Z+ is back, Y+ is top, Y- is bottom, X+ is left, X- is right
    let look_x = 0f32;
    let look_y = 4f32;
    let look_z = 0f32;

    let view = glam::Mat4::look_at_rh(
        camera,
        Vec3::new(look_x, look_y, look_z),
        Vec3::Y,
    );

    println!("Camera: {:?}", camera);

    projection * view
}