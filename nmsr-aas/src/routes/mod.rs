pub mod bbmodel_export;
pub mod extractors;
pub mod query;
mod render;
mod render_model;
mod render_skin;
use crate::{
    config::{FeaturesConfiguration, ModelCacheConfiguration, NmsrConfiguration},
    error::Result,
    model::{
        armor::manager::VanillaMinecraftArmorManager,
        request::{
            cache::{CacheBias, ModelCache},
            entry::RenderRequestEntry,
            RenderRequest, RenderRequestFeatures, RenderRequestMode,
        },
        resolver::{
            default_skins::DefaultSkin, mojang::client::MojangClient, RenderRequestResolver,
            ResolvedRenderRequest,
        },
    },
};
use deadpool::managed::Object;
use enumset::EnumSet;
use image::RgbaImage;
use nmsr_rendering::high_level::camera::Camera;
use nmsr_rendering::high_level::pipeline::{
    pools::SceneContextPoolManager, Backends, Features, GraphicsContext, GraphicsContextDescriptor,
    GraphicsContextPools,
};
pub use render::{render, render_get_warning, render_post_warning};
#[cfg(feature = "renderdoc")]
use {renderdoc::{RenderDoc, V141}, tokio::sync::Mutex};
use std::{borrow::Cow, sync::Arc, time::Duration};
use strum::IntoEnumIterator;
use tracing::{info, info_span, instrument, Instrument, Span};

pub trait RenderRequestValidator {
    fn validate_mode(&self, mode: &RenderRequestMode) -> bool;

    #[allow(unused_variables)]
    fn cleanup_request(&self, request: &mut RenderRequest) {}
}

#[derive(Clone)]
pub struct NMSRState<'a> {
    pub resolver: Arc<RenderRequestResolver>,
    pub armor_manager: Option<Arc<VanillaMinecraftArmorManager>>,
    pub graphics_context: Arc<GraphicsContext<'a>>,
    pools: Arc<GraphicsContextPools<'a>>,
    cache_config: ModelCacheConfiguration,
    features_config: FeaturesConfiguration,
    #[cfg(feature = "renderdoc")]
    pub render_doc: Arc<Mutex<RenderDoc<V141>>>,
}

impl<'a> RenderRequestValidator for NMSRState<'a> {
    fn validate_mode(&self, mode: &RenderRequestMode) -> bool {
        !self.features_config.disabled_modes.contains(mode)
    }

    fn cleanup_request(&self, request: &mut RenderRequest) {
        let mut disabled_features: EnumSet<RenderRequestFeatures> = EnumSet::new();
        for feature in self.features_config.disabled_features.iter() {
            disabled_features.insert(*feature);
        }

        if disabled_features.contains(RenderRequestFeatures::ExtraSettings) {
            request.extra_settings = None;
        }

        request.features.remove_all(disabled_features);

        if let (None, Some(extra)) = (&self.armor_manager, &mut request.extra_settings) {
            extra.helmet.take();
            extra.chestplate.take();
            extra.leggings.take();
            extra.boots.take();
        }
    }
}

impl<'a> NMSRState<'a> {
    const ONE_YEAR_DURATION: Duration = Duration::from_secs(
        60 /* seconds */ * 60 /* minutes */ * 24 /* hours */ * 365, /* days */
    );

    pub async fn new(config: &NmsrConfiguration) -> Result<Self> {
        let mojang_client = MojangClient::new(Arc::new(config.mojank.clone()))?;
        let cache_config = Self::setup_default_skin_cache_biases(config.caching.clone());
        let model_cache = ModelCache::new("cache".into(), cache_config).await?;

        let rendering_config = config.rendering.clone();

        let resolver = RenderRequestResolver::new(model_cache, Arc::new(mojang_client));

        #[cfg(feature = "renderdoc")]
        let mut rd: RenderDoc<V141> = RenderDoc::new()?;

        #[cfg(feature = "renderdoc")]
        {
            rd.set_capture_file_path_template("captures/nmsr");
        }

        let graphics_context = GraphicsContext::new(GraphicsContextDescriptor {
            backends: Some(Backends::all()),
            surface_provider: Box::new(|_| None),
            default_size: (0, 0), // can be zero since we don't provide any surface
            texture_format: None,
            features: Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            limits: None,
            blend_state: None,
            sample_count: rendering_config.as_ref().map(|c| c.sample_count),
            use_smaa: rendering_config.as_ref().map(|c| c.use_smaa),
        })
        .await?;

        let graphics_context = Arc::new(graphics_context);

        let pools = GraphicsContextPools::new(graphics_context.clone())?;

        let armor_manager = if config
            .features
            .as_ref()
            .is_some_and(|f| f.disable_armor_rendering)
        {
            None
        } else {
            Some(Arc::new(
                VanillaMinecraftArmorManager::new("cache".into()).await?,
            ))
        };

        Ok(Self {
            resolver: Arc::new(resolver),
            graphics_context,
            pools: Arc::new(pools),
            cache_config: config.caching.clone(),
            armor_manager,
            features_config: config.features.clone().unwrap_or_default(),
            #[cfg(feature = "renderdoc")]
            render_doc: Arc::new(Mutex::new(rd)),
        })
    }

    pub async fn create_scene_context(&self) -> Result<Object<SceneContextPoolManager<'a>>> {
        Ok(self.pools.create_scene_context().await?)
    }

    #[allow(unused_variables)]
    #[cfg_attr(not(feature = "ears"), allow(clippy::unnecessary_wraps))]
    pub fn process_skin(
        skin_image: RgbaImage,
        features: EnumSet<RenderRequestFeatures>,
    ) -> Result<RgbaImage> {
        let mut skin_image = ears_rs::utils::upgrade_skin_if_needed(skin_image);

        #[cfg(feature = "ears")]
        {
            if features.contains(RenderRequestFeatures::Ears) {
                ears_rs::utils::process_erase_regions(&mut skin_image)?;
            }
        }

        ears_rs::utils::strip_alpha(&mut skin_image);

        Ok(skin_image)
    }

    #[cfg(feature = "ears")]
    pub fn apply_ears_camera_settings(
        features: &ears_rs::features::EarsFeatures,
        mode: RenderRequestMode,
        request_features: &EnumSet<RenderRequestFeatures>,
        camera: &mut Camera,
    ) {
        use ears_rs::features::data::ear::EarMode;
        let mut look_at_y_offset: f32 = 0.0;
        let mut distance_offset: f32 = 0.0;

        if features.ear_mode == EarMode::Around || features.ear_mode == EarMode::Above {
            look_at_y_offset += 2.5;
            distance_offset += 3.5;

            if !mode.is_isometric() {
                distance_offset += 4.0;
            } else if !mode.is_front() {
                look_at_y_offset += 0.25;
            }

            if mode.is_front() && !mode.is_face() {
                distance_offset -= 1.25;
            }

            if mode.is_isometric() && mode.is_full_body() {
                distance_offset -= 1.0;
            }

            if mode.is_face() {
                look_at_y_offset += 1.5;
                distance_offset += 0.25;
            }
        }

        if request_features.contains(RenderRequestFeatures::FlipUpsideDown) {
            look_at_y_offset *= -1.0;
        }

        camera.set_look_at_y(camera.get_look_at_y() + look_at_y_offset);

        camera.set_aspect(camera.get_aspect() + distance_offset);
        camera.set_distance(camera.get_distance() + distance_offset);
    }

    pub fn apply_deadmau5ears_camera_settings(
        mode: RenderRequestMode,
        request_features: &EnumSet<RenderRequestFeatures>,
        camera: &mut Camera,
    ) {
        let mut look_at_y_offset: f32 = 2.5f32;
        let mut distance_offset: f32 = 3.5f32;

        distance_offset += match mode {
            RenderRequestMode::Face => 0.75,
            RenderRequestMode::FullBodyIso | RenderRequestMode::FrontFull => -1.0,
            RenderRequestMode::HeadIso => -0.5,
            RenderRequestMode::FullBody | RenderRequestMode::BodyBust => 2.0,
            RenderRequestMode::FrontBust => -2.5,
            RenderRequestMode::Head => 5.0,
            _ => 0.0,
        };

        if request_features.contains(RenderRequestFeatures::FlipUpsideDown) {
            look_at_y_offset *= -1.0;
            look_at_y_offset += 1.0;
        }

        camera.set_look_at_y(camera.get_look_at_y() + look_at_y_offset);

        camera.set_aspect(camera.get_aspect() + distance_offset);
        camera.set_distance(camera.get_distance() + distance_offset);
    }

    fn apply_upside_down_camera_settings(mode: RenderRequestMode, camera: &mut Camera) {
        if mode.is_bust() {
            camera.set_look_at_y(camera.get_look_at_y() - 16.0);
        } else if mode.is_head_or_face() {
            camera.set_look_at_y(4.0);
        } else {
            camera.set_look_at_y(camera.get_look_at_y() - 1.0);
        }
    }

    #[instrument(skip(self))]
    pub(crate) async fn init(&self) -> Result<()> {
        info!("Pre-loading our cache biases.");
        self.preload_cache_biases().await?;

        info!("Starting cache clean-up task");
        self.start_cache_cleanup_task();

        Ok(())
    }

    fn start_cache_cleanup_task(&self) {
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
    }

    #[inline]
    #[instrument(name = "clean_cache", skip_all)]
    async fn do_cache_clean_up(resolver: Arc<RenderRequestResolver>) -> Result<()> {
        resolver.do_cache_clean_up().await
    }

    #[instrument(skip(self))]
    async fn preload_cache_biases(&self) -> Result<()> {
        #[inline]
        async fn resolve_entry(
            resolver: &RenderRequestResolver,
            entry: RenderRequestEntry,
        ) -> Result<ResolvedRenderRequest> {
            let request = RenderRequest::new_from_excluded_features(
                RenderRequestMode::Skin,
                entry,
                None,
                EnumSet::empty(),
                None,
            );

            resolver.resolve(&request).await
        }

        let resolve_cache_biases_span = info_span!("resolve_cache_biases");

        let (entries, resolved_entries) = async_scoped::TokioScope::scope_and_block(|s| {
            for entry in self.cache_config.cache_biases.keys() {
                s.spawn(
                    resolve_entry(&self.resolver, entry.clone())
                        .instrument(Span::none())
                        .instrument(resolve_cache_biases_span.clone()),
                );
            }

            self.cache_config.cache_biases.keys().take(5).cloned()
        });

        info!("Resolved all cache biases, prewarming renderer now.");

        let prewarm_renderer_span = info_span!("prewarm_renderer");

        async_scoped::TokioScope::scope_and_block(|s| {
            for result in resolved_entries.into_iter().zip(entries) {
                let (Ok(Ok(resolved)), entry) = result else {
                    continue;
                };

                s.spawn(
                    async move {
                        // Prewarm our renderer by actually rendering a few requests.
                        // This will ensure that the renderer is initialized and ready to go when we start serving requests.
                        let request = RenderRequest::new_from_excluded_features(
                            RenderRequestMode::FullBody,
                            entry,
                            None,
                            EnumSet::empty(),
                            None,
                        );

                        for mode in
                            RenderRequestMode::iter().filter(|m| m.uses_rendering_pipeline())
                        {
                            let mut req = request.clone();
                            req.mode = mode;

                            let resolved = resolved.clone();

                            drop(
                                render_model::internal_render_model(&mut req, &self, &resolved)
                                    .instrument(Span::none())
                                    .await,
                            )
                        }
                    }
                    .instrument(prewarm_renderer_span.clone()),
                );
            }
        });

        Ok(())
    }

    fn setup_default_skin_cache_biases(
        mut config: ModelCacheConfiguration,
    ) -> ModelCacheConfiguration {
        for skin in DefaultSkin::iter() {
            for slim in [true, false].into_iter() {
                let entry = RenderRequestEntry::default_skin_hash(skin, slim);

                config
                    .cache_biases
                    .insert(entry, CacheBias::CacheIndefinitely);
            }
        }

        return config;
    }

    pub fn get_cache_control_for_request(&self, request: &RenderRequest) -> Cow<'_, str> {
        // Don't cache requests using custom mode.
        if request.mode.is_custom() {
            return "public, no-store".into();
        }

        // Get the cache duration for this entry.
        let entry_duration = self.cache_config.get_cache_duration(&request.entry);

        // Limit our max-age duration to 1 year if we have set this entry to be cached forever.
        let max_age_duration = entry_duration.min(&Self::ONE_YEAR_DURATION);

        let immutable = if entry_duration == &Duration::MAX {
            ", immutable"
        } else {
            ""
        };

        let max_age = max_age_duration.as_secs();

        format!("public, max-age={max_age}{immutable}").into()
    }
}
