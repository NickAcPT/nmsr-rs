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
    
    let mut texture = image::open("NickAc.png").unwrap().into_rgba8();
    
    ears_rs::utils::strip_alpha(&mut texture);
    
    let state = ShaderState {
        transform: camera.get_view_projection_matrix(),
        texture,
        sun: shader::SunInformation {
            direction: glam::Vec3::new(0.0, -1.0, 1.0),
            intensity: 2.0,
            ambient: 0.621,
        },
    };

    entry.draw(&state);

    entry.dump();
}
