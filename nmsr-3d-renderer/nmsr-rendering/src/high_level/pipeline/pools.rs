use std::sync::Arc;

use deadpool::managed::{Manager, RecycleResult,Metrics};
use tracing::trace;

use crate::errors::NMSRRenderingError;

use super::{scene_context::SceneContext, GraphicsContext};

pub struct SceneContextPoolManager<'a> {
    graphics_context: Arc<GraphicsContext<'a>>,
}

impl<'a> SceneContextPoolManager<'a> {
    pub fn new(graphics_context: Arc<GraphicsContext<'a>>) -> Self {
        Self { graphics_context }
    }
}

impl<'a> std::fmt::Debug for SceneContextPoolManager<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneContextPoolManager").finish()
    }
}

impl<'a> Manager for SceneContextPoolManager<'a> {
    type Type = SceneContext;
    type Error = Box<NMSRRenderingError>;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        trace!("Creating new scene context");
        Ok(SceneContext::new(&self.graphics_context))
    }

    async fn recycle(&self, obj: &mut Self::Type, _metrics: &Metrics) -> RecycleResult<Self::Error> {
        // If for some reason the smaa target is is no longer present, we're gonna rip
        // the textures out of the scene context so that the smaa target can be recreated.
        if obj.smaa_target.is_none() {
            obj.textures.take();
        }
        
        trace!(metrics = ?_metrics, "Recycling existing scene context");
        
        Ok(())
    }
}
