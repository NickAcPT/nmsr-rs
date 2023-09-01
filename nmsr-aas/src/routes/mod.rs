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
use tracing::{debug_span, info, info_span, instrument, Instrument};
use uuid::uuid;

use std::{hint::black_box, sync::Arc};

use crate::{
    config::{ModelCacheConfiguration, NmsrConfiguration},
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
    cache_config: ModelCacheConfiguration,
}

impl NMSRState {
    pub async fn new(config: &NmsrConfiguration) -> Result<Self> {
        let mojang_client = MojangClient::new(Arc::new(config.mojank.clone()))?;
        let cache_config = config.caching.clone();
        let model_cache = ModelCache::new("cache".into(), cache_config).await?;

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
            cache_config: config.caching.clone(),
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

    #[instrument(skip(self))]
    pub(crate) async fn init(&self) -> Result<()> {
        info!("Pre-loading our cache biases.");
        self.preload_cache_biases().await?;

        #[cfg(not(debug_assertions))]
        {
            info!("Pre-warming model renderer.");
            self.prewarm_renderer().await?;
        }

        info!("Starting cache clean-up task");
        self.start_cache_cleanup_task()?;

        Ok(())
    }

    fn start_cache_cleanup_task(&self) -> Result<()> {
        let mut interval = tokio::time::interval(self.cache_config.cleanup_interval);

        let resolver = self.resolver.clone();

        tokio::task::spawn(async move {
            loop {
                interval.tick().await;

                if let Err(err) = Self::do_cache_clean_up(resolver.clone()).await {
                    tracing::error!("Error while cleaning up cache: {:?}", err);
                }
            }
        });

        Ok(())
    }

    #[inline]
    #[instrument(name = "clean_cache", skip_all)]
    async fn do_cache_clean_up(resolver: Arc<RenderRequestResolver>) -> Result<()> {
        resolver.do_cache_clean_up().await
    }

    #[instrument(skip(self))]
    async fn preload_cache_biases(&self) -> Result<()> {
        for (entry, _) in &self.cache_config.cache_biases {
            let _guard = debug_span!("preload_cache_biases", entry = ?entry).entered();

            let request = RenderRequest::new_from_excluded_features(
                RenderRequestMode::Skin,
                entry.clone(),
                None,
                EnumSet::EMPTY,
                None,
            );

            self.resolver.resolve(&request).await?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn prewarm_renderer(&self) -> Result<()> {
        // Prewarm our renderer by actually rendering a few requests.
        // This will ensure that the renderer is initialized and ready to go when we start serving requests.
        let entry =
            RenderRequestEntry::MojangPlayerUuid(uuid!("ad4569f3-7576-4376-a7c7-8e8cfcd9b832"));
        let request = RenderRequest::new_from_excluded_features(
            RenderRequestMode::FullBody,
            entry,
            None,
            EnumSet::EMPTY,
            None,
        );

        let resolved = self.resolver.resolve(&request).await?;

        for index in 0..15 {
            let _ = black_box(
                black_box(render_model::internal_render_model(
                    &request, self, &resolved,
                ))
                .instrument(info_span!("prewarm_render", index = index))
                .await?,
            );
        }

        Ok(())
    }
}
