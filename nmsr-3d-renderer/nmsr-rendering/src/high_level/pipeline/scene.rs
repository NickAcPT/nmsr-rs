use crate::high_level::camera::Camera;
use crate::high_level::pipeline::scene_internal::SceneWgpuInternal;
use crate::high_level::pipeline::wgpu_pipeline::NmsrWgpuPipeline;
use crate::low_level::primitives::part_primitive::PartPrimitive;

pub struct Size {
    pub width: u32,
    pub height: u32,
}

pub struct Scene {
    pub camera: Camera,
    viewport_size: Size,
    objects: Vec<Box<dyn PartPrimitive>>,
    wgpu_internal: SceneWgpuInternal,
}

impl Scene {
    pub fn new(
        pipeline: NmsrWgpuPipeline,
        camera: Camera,
        viewport_size: Size,
        objects: Vec<Box<dyn PartPrimitive>>,
    ) -> Self {
        let internal = SceneWgpuInternal::new(pipeline, &viewport_size);

        Self {
            camera,
            viewport_size,
            objects,
            wgpu_internal: internal,
        }
    }
}
