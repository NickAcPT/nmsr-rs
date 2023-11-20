use glam::Mat4;

use crate::{model::RenderEntry, shader::ShaderState};

mod model;
mod camera;
pub mod shader;
mod logic;

fn main() {
    let mut entry = RenderEntry::new((100, 100).into());
    
    let state = ShaderState {
        transform: Mat4::IDENTITY,
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
