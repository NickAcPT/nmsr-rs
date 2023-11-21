#![feature(core_intrinsics)]

use std::fs;

use glam::{Mat4, Vec3};

pub use crate::{camera::CameraRotation, model::{RenderEntry, Size}, shader::ShaderState};

pub mod camera;
pub mod logic;
pub mod model;
pub mod shader;


fn main() {

    let mut camera = camera::Camera::new_orbital(
        Vec3::new(0.0, 16.5 + 2.5, 0.0),
        45.0 + 3.5 + 4.0,
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
    
    ears_rs::utils::process_erase_regions(&mut texture).expect("Failed to process erase regions");
    ears_rs::utils::strip_alpha(&mut texture);
    
    fs::create_dir("output").unwrap_or_default();
    
    let mut state = ShaderState {
        transform: camera.get_view_projection_matrix(),
        texture,
        sun: shader::SunInformation {
            direction: glam::Vec3::new(0.0, -1.0, 1.0),
            intensity: 2.0,
            ambient: 0.621,
        },
    };
    
    for angle in 0..360 {
        let mut entry = RenderEntry::new((512, 869).into());
        camera.get_rotation_mut().yaw = angle as f32;
        
        state.transform = camera.get_view_projection_matrix();
        // Measure draw
        let start = std::time::Instant::now();
        entry.draw(&state);
        let end = std::time::Instant::now();
        println!("Draw took {}ms", (end - start).as_millis());

        entry.textures.output.save(format!("output/output-{angle}.png")).unwrap();
    }

}
