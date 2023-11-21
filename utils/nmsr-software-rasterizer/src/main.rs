use glam::{Mat4, Vec3};

use crate::{camera::CameraRotation, model::{RenderEntry, Size}, shader::ShaderState};

mod camera;
mod logic;
mod model;
pub mod shader;

fn main() {
    let mut entry = RenderEntry::new((512, 869).into());

    let mut camera = camera::Camera::new_orbital(
        Vec3::new(0.0, 16.5, 0.0),
        45.0,
        CameraRotation {
            yaw: 20f32,
            pitch: 10f32,
            roll: 0f32,
        },
        camera::ProjectionParameters::Perspective { fov: 45f32 },
        Some(Size {
            width: 512,
            height: 869
        }),
    );
    
    let state = ShaderState {
        transform: camera.get_view_projection_matrix(),
        texture: image::open("NickAc.png").unwrap().into_rgba8(),
        sun: shader::SunInformation {
            direction: glam::Vec3::ZERO,
            intensity: 1.0,
            ambient: 1.0,
        },
    };

    entry.draw(&state);

    entry.dump();
}
