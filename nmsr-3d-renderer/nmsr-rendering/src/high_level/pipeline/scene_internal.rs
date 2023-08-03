use crate::high_level::pipeline::scene::Size;
use crate::high_level::pipeline::wgpu_pipeline::NmsrWgpuPipeline;

pub struct SceneWgpuInternal {
    pipeline: NmsrWgpuPipeline,
}

impl SceneWgpuInternal {
    pub fn new(pipeline: NmsrWgpuPipeline, viewport_size: &Size) -> Self {
        unimplemented!()
    }
}
