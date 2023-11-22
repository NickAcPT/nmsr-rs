#![feature(core_intrinsics)]

use std::{fs, hint::black_box};

use ears_rs::parser::EarsParser;
use glam::Vec3;
use nmsr_rendering::high_level::parts::provider::PlayerPartProviderContext;

pub use crate::{camera::CameraRotation, model::{RenderEntry, Size}, shader::ShaderState};

pub mod camera;
pub mod logic;
pub mod model;
pub mod shader;

fn main() {

    let mut camera = camera::Camera::new_orbital(
        Vec3::new(0.0, 16.5 + 2.5, 0.0),
        5.0 + 3.5 + 4.0,
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
    
    let context: PlayerPartProviderContext<()> = PlayerPartProviderContext {
        model: nmsr_rendering::high_level::model::PlayerModel::Alex,
        has_hat_layer: true,
        has_layers: true,
        has_cape: false,
        arm_rotation: 10.0,
        shadow_y_pos: None,
        shadow_is_square: false,
        armor_slots: None,
        ears_features: EarsParser::parse(&texture).expect("Yes"),
    };
    
    ears_rs::utils::process_erase_regions(&mut texture).expect("Failed to process erase regions");
    ears_rs::utils::strip_alpha(&mut texture);
    
    fs::create_dir("output").unwrap_or_default();
    
    let mut state = ShaderState::new(camera, texture, shader::SunInformation {
        direction: glam::Vec3A::new(0.0, -1.0, 1.0),
        intensity: 2.0,
        ambient: 0.621,
    });
    
    let mut entry = RenderEntry::new((512, 869).into(), &context);
    
    for angle in 0..360 {
        entry.textures.output.fill(0);
        entry.textures.depth_buffer.fill(1.0);
        
        camera.get_rotation_mut().yaw = angle as f32;
        state.update();
        
        entry.draw(black_box(&state));
    }
}
