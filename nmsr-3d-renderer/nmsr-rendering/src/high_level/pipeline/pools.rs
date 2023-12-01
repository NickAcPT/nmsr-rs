use std::sync::Arc;

use async_trait::async_trait;
use deadpool::managed::{Manager, RecycleResult,Metrics};

use crate::errors::NMSRRenderingError;

use super::{scene_context::SceneContext, GraphicsContext};

pub struct SceneContextPoolManager {
    graphics_context: Arc<GraphicsContext>,
}

impl SceneContextPoolManager {
    pub fn new(graphics_context: Arc<GraphicsContext>) -> Self {
        Self { graphics_context }
    }
}

impl std::fmt::Debug for SceneContextPoolManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneContextPoolManager").finish()
    }
}

#[async_trait]
impl Manager for SceneContextPoolManager {
    type Type = SceneContext;
    type Error = Box<NMSRRenderingError>;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        Ok(SceneContext::new(&self.graphics_context))
    }

    async fn recycle(&self, obj: &mut Self::Type, _metrics: &Metrics) -> RecycleResult<Self::Error> {
        // If for some reason the smaa target is is no longer present, we're gonna rip
        // the textures out of the scene context so that the smaa target can be recreated.
        if obj.smaa_target.is_none() {
            obj.textures.take();
        }
        
        Ok(())
    }
}
