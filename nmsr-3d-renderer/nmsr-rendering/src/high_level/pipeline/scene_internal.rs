use crate::high_level::pipeline::scene::Size;
use crate::high_level::pipeline::wgpu_pipeline::GraphicsContext;

pub struct SceneWgpuInternal {
    pipeline: GraphicsContext,
}

impl SceneWgpuInternal {
    pub fn new(pipeline: GraphicsContext, viewport_size: &Size) -> Self {
        unimplemented!()
    }
}
