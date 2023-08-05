use std::sync::Arc;

use crate::high_level::camera::Camera;
use crate::high_level::pipeline::SceneContext;

#[derive(Copy, Clone)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

pub struct Scene {
    pub camera: Camera,
    viewport_size: Size,
    scene_context: Arc<SceneContext>,
}

impl Scene {
    pub fn new(
        context: Arc<SceneContext>,
        mut camera: Camera,
        viewport_size: Size,
    ) -> Self {
        // Initialize our camera with the viewport size
        camera.set_aspect_ratio(viewport_size.width as f32 / viewport_size.height as f32);

        Self {
            camera,
            viewport_size,
            scene_context: context,
        }
    }
}
