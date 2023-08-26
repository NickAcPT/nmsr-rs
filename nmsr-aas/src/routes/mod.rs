mod render_model;
pub mod extractors;

use nmsr_rendering::high_level::pipeline::{GraphicsContext, GraphicsContextDescriptor, Backends};
pub use render_model::render_model;

use std::sync::Arc;

use crate::{
    config::NmsrConfiguration,
    error::Result,
    model::{
        request::cache::ModelCache,
        resolver::{mojang::client::MojangClient, RenderRequestResolver},
    },
};

#[derive(Clone)]
pub struct NMSRState {
    pub resolver: Arc<RenderRequestResolver>,
    pub graphics_context: Arc<GraphicsContext>
}

impl NMSRState {
    pub async fn new(config: &NmsrConfiguration) -> Result<Self> {
        let mojang_client = MojangClient::new(Arc::new(config.mojank.clone()))?;
        let model_cache = ModelCache::new("cache".into(), config.caching.clone())?;

        let resolver = RenderRequestResolver::new(model_cache, Arc::new(mojang_client));

        let graphics_context = GraphicsContext::new(GraphicsContextDescriptor {
            backends: Some(Backends::all()),
            surface_provider: Box::new(|_| None),
            default_size: (0, 0), // can be zero since we don't provide any surface
            texture_format: None,
        }).await?;
        
        Ok(Self {
            resolver: Arc::new(resolver),
            graphics_context: Arc::new(graphics_context)
        })
    }
}
