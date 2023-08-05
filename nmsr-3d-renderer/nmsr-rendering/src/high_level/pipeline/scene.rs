use std::sync::Arc;

use crate::high_level::camera::Camera;
use crate::high_level::pipeline::SceneContext;

#[derive(Copy, Clone)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

pub struct Scene<'ctx> {
    pub camera: Camera,
    viewport_size: Size,
    scene_context: SceneContext<'ctx>,
}

impl<'ctx> Scene<'ctx> {
    pub fn new(
        context: SceneContext<'ctx>,
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

    pub fn get_context(&'ctx mut self) -> &'ctx mut SceneContext<'ctx> {
        &mut self.scene_context
    }
}
