pub mod extractors;
mod render;
mod render_model;
mod render_skin;

use deadpool::managed::Object;
use enumset::EnumSet;
use image::RgbaImage;
use nmsr_rendering::high_level::pipeline::{
    pools::SceneContextPoolManager, Backends, GraphicsContext, GraphicsContextDescriptor,
    GraphicsContextPools,
};
pub use render::render;
use tracing::{info_span, instrument, Span};

use std::{hint::black_box, sync::Arc};

use crate::{
    config::NmsrConfiguration,
    error::{RenderRequestError, Result},
    model::{
        request::{
            cache::ModelCache, entry::RenderRequestEntry, RenderRequest, RenderRequestFeatures,
            RenderRequestMode,
        },
        resolver::{mojang::client::MojangClient, RenderRequestResolver},
    },
};

#[derive(Clone)]
pub struct NMSRState {
    pub resolver: Arc<RenderRequestResolver>,
    pub graphics_context: Arc<GraphicsContext>,
    pools: Arc<GraphicsContextPools>,
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
        })
        .await?;

        let graphics_context = Arc::new(graphics_context);

        let pools = GraphicsContextPools::new((&graphics_context).clone())?;

        Ok(Self {
            resolver: Arc::new(resolver),
            graphics_context,
            pools: Arc::new(pools),
        })
    }

    pub async fn create_scene_context(&self) -> Result<Object<SceneContextPoolManager>> {
        Ok(self.pools.create_scene_context().await?)
    }

    #[allow(unused_variables)]
    pub fn process_skin(
        &self,
        skin_image: RgbaImage,
        features: EnumSet<RenderRequestFeatures>,
    ) -> Result<RgbaImage> {
        let mut skin_image = ears_rs::utils::legacy_upgrader::upgrade_skin_if_needed(skin_image)
            .ok_or(RenderRequestError::LegacySkinUpgradeError)?;

        #[cfg(feature = "ears")]
        {
            if features.contains(RenderRequestFeatures::Ears) {
                ears_rs::utils::eraser::process_erase_regions(&mut skin_image)?;
            }
        }

        ears_rs::utils::alpha::strip_alpha(&mut skin_image);

        Ok(skin_image)
    }

    // Prewarm our renderer by actually rendering a few requests.
    // This will ensure that the renderer is initialized and ready to go when we start serving requests.
    #[instrument(skip(self))]
    pub(crate) async fn prewarm_renderer(&self) -> Result<()> {
        // `86ed67a77cf4e00350b6e3a966f312d4f5a0170a028c0699e6043a2374f99ff5` is one of the hashes of NickAc's skin.
        let entry = RenderRequestEntry::TextureHash(
            "86ed67a77cf4e00350b6e3a966f312d4f5a0170a028c0699e6043a2374f99ff5".to_owned(),
        );
        let request = RenderRequest::new_from_excluded_features(
            RenderRequestMode::FullBody,
            entry,
            None,
            EnumSet::EMPTY,
            None,
        );

        let resolved = self.resolver.resolve(&request).await?;

        for index in 0..50 {
            let result = info_span!("prewarm_render", index = index).in_scope(|| {
                black_box(
                    render_skin::internal_render_skin(request.clone(), self, resolved.clone()),
                )
            }).await;
            drop(result);
        }

        Ok(())
    }
}
