
use std::collections::HashMap;

use nmsr_player_parts::types::PlayerPartTextureType;

use crate::high_level::camera::Camera;
use crate::high_level::pipeline::SceneContext;

use super::{GraphicsContext, SceneTexture};

#[derive(Copy, Clone)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

pub struct Scene {
    pub camera: Camera,
    viewport_size: Size,
    scene_context: SceneContext,
    textures: HashMap<PlayerPartTextureType, SceneTexture>
}

impl Scene {
    pub fn new(
        graphics_context: &GraphicsContext,
        mut scene_context: SceneContext,
        mut camera: Camera,
        viewport_size: Size,
    ) -> Self {
        // Initialize our camera with the viewport size
        camera.set_aspect_ratio(viewport_size.width as f32 / viewport_size.height as f32);
        
        scene_context.init(graphics_context, &mut camera, viewport_size);
        
        Self {
            camera,
            viewport_size,
            scene_context,
            textures: HashMap::new()
        }
    }
}
