use std::sync::Arc;

use async_trait::async_trait;
use deadpool::managed::{Manager, RecycleResult};

use crate::errors::NMSRRenderingError;

use super::{scene_context::SceneContext, GraphicsContext};

pub struct SceneContextPoolManager {
    graphics_context: Arc<GraphicsContext>,
}

impl SceneContextPoolManager {
    pub fn new(graphics_context: Arc<GraphicsContext>) -> Self { Self { graphics_context } }
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

    async fn recycle(&self, _: &mut Self::Type) -> RecycleResult<Self::Error> {
        Ok(())
    }
}
