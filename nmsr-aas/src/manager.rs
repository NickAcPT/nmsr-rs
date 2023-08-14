use std::sync::Arc;

#[cfg(feature = "renderdoc")]
use std::sync::Mutex;

use enumset::EnumSet;
use nmsr_rendering::{
    high_level::pipeline::Backends,
    high_level::{
        camera::{Camera, CameraRotation, ProjectionParameters},
        pipeline::{scene::SunInformation, GraphicsContext, GraphicsContextDescriptor},
        types::PlayerBodyPartType,
    },
};

use strum::{Display, EnumCount, EnumIter, EnumString, IntoEnumIterator};
#[cfg(feature = "uv")]
use {
    crate::utils::errors::NMSRaaSError::MissingPartManager, std::borrow::Cow,
    std::collections::HashMap, std::path::Path, strum::IntoEnumIterator, tracing::instrument,
};

#[cfg(feature = "lazy_parts")]
use tracing::debug;

#[cfg(feature = "uv")]
use nmsr_lib::{
    parts::manager::PartsManager,
    vfs::{PhysicalFS, VfsPath},
};
use tracing::{info, instrument};
#[cfg(feature = "lazy_parts")]
use {
    crate::utils::errors::NMSRaaSError,
    rayon::prelude::*,
    std::io::{BufReader, BufWriter, Write},
};

use crate::{renderer, utils::Result};

#[cfg(feature = "wgpu")]
use crate::model::{resolver::RenderRequestResolver, RenderRequest, RenderRequestEntry};

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString, EnumIter, EnumCount, Display)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum RenderMode {
    FullBody,
    FrontFull,
    #[cfg(feature = "wgpu")]
    Bust,
    FullBodyIso,
    Head,
    HeadIso,
    Face,
}

#[derive(Debug, Clone)]
#[cfg(feature = "uv")]
pub(crate) struct NMSRaaSManager {
    #[cfg(feature = "lazy_parts")]
    part_root: VfsPath,
    #[cfg(not(feature = "lazy_parts"))]
    part_managers: HashMap<RenderMode, PartsManager>,
}

#[cfg(feature = "wgpu")]
impl RenderMode {
    pub(crate) fn get_camera(&self) -> Camera {
        let look_at = [0.0, 16.5, 0.0].into();

        match self {
            RenderMode::FullBody => Camera::new_orbital(
                look_at,
                45.0,
                CameraRotation {
                    yaw: 25.0,
                    pitch: 11.5,
                },
                ProjectionParameters::Perspective { fov: 45.0 },
                1.0,
            ),
            RenderMode::FullBodyIso => Camera::new_orbital(
                look_at,
                45.0,
                CameraRotation {
                    yaw: 45.0,
                    pitch: std::f32::consts::FRAC_1_SQRT_2.atan().to_degrees(),
                },
                ProjectionParameters::Orthographic { aspect: 17.0 },
                1.0,
            ),
            _ => unimplemented!("wgpu rendering is not yet implemented"),
        }
    }

    pub(crate) fn get_lighting(&self, no_shading: bool) -> SunInformation {
        if no_shading {
            return SunInformation::new([0.0; 3].into(), 0.0, 1.0);
        } else {
            match self {
                RenderMode::FullBody | RenderMode::FullBodyIso => {
                    SunInformation::new([0.0, -1.0, 5.0].into(), 1.0, 0.7)
                }
                _ => SunInformation::new([0.0; 3].into(), 0.0, 1.0),
            }
        }
    }

    pub(crate) fn get_arm_rotation(&self) -> f32 {
        match self {
            RenderMode::FullBody => 10.0,
            _ => 0.0,
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub(crate) fn get_body_parts(&self) -> Vec<PlayerBodyPartType> {
        match self {
            RenderMode::FullBody | RenderMode::FrontFull | RenderMode::FullBodyIso => {
                PlayerBodyPartType::iter().collect()
            }
            RenderMode::Head | RenderMode::HeadIso | RenderMode::Face => {
                vec![PlayerBodyPartType::Head, PlayerBodyPartType::HeadLayer]
            }
            RenderMode::Bust => {
                let excluded = vec![PlayerBodyPartType::LeftLeg, PlayerBodyPartType::RightLeg];

                PlayerBodyPartType::iter()
                    .filter(|m| excluded.contains(&m.get_non_layer_part()))
                    .collect()
            }
        }
    }
}

#[cfg(feature = "wgpu")]
#[derive(Debug)]
pub(crate) struct NMSRaaSManager {
    pub graphics_context: GraphicsContext,

    #[cfg(feature = "renderdoc")]
    pub renderdoc: Arc<Mutex<renderdoc::RenderDoc<renderdoc::V140>>>,
}

#[cfg(feature = "wgpu")]
impl NMSRaaSManager {
    pub(crate) async fn new() -> Result<NMSRaaSManager> {
        #[cfg(feature = "renderdoc")]
        let renderdoc =
            renderdoc::RenderDoc::<renderdoc::V140>::new().expect("Failed to initialize RenderDoc");

        // Setup an nmsr wgpu rendering pipeline.
        // Since we are not rendering to a surface (i.e. a window), we don't need to provide
        // a surface, nor a default size.
        let graphics_context = GraphicsContext::new(GraphicsContextDescriptor {
            backends: Some(Backends::all()),
            surface_provider: Box::new(|_| None),
            default_size: (0, 0),
            texture_format: Some(GraphicsContext::DEFAULT_TEXTURE_FORMAT),
        })
        .await?;

        info!(
            "Created graphics context with adapter {:?} and we're using {} MSAA samples.",
            &graphics_context.adapter.get_info(),
            graphics_context.sample_count
        );

        #[cfg(feature = "renderdoc")]
        renderdoc
            .launch_replay_ui(true, None)
            .expect("Failed to launch RenderDoc replay UI");

        let manager = NMSRaaSManager {
            graphics_context,
            #[cfg(feature = "renderdoc")]
            renderdoc: Arc::new(Mutex::new(renderdoc)),
        };

        Ok(manager)
    }

    // Pre-warm the graphics context by rendering a single skin.
    pub(crate) async fn pre_warm(&self, resolver: Arc<RenderRequestResolver>) -> Result<()> {
        use crate::model::RenderRequestEntryModel;

        // Generate a render request entry for NickAc (ad4569f3-7576-4376-a7c7-8e8cfcd9b832)
        let model = RenderRequestEntry::TextureHash(
            "86ed67a77cf4e00350b6e3a966f312d4f5a0170a028c0699e6043a2374f99ff5".to_owned(),
        );
        let request = RenderRequest::new_from_excluded_features(
            model,
            Some(RenderRequestEntryModel::Alex),
            EnumSet::EMPTY,
        );

        let render = resolver.resolve(request).await?;

        let _ = renderer::render_skin(self, &RenderMode::FullBody, render, false, true).await;

        Ok(())
    }

    #[cfg(feature = "renderdoc")]
    pub(crate) fn start_frame_capture(&self) {
        self.renderdoc
            .lock()
            .expect("RenderDocMut")
            .start_frame_capture(std::ptr::null(), std::ptr::null());
    }

    #[cfg(feature = "renderdoc")]
    pub(crate) fn end_frame_capture(&self) {
        self.renderdoc
            .lock()
            .expect("RenderDocMut")
            .end_frame_capture(std::ptr::null(), std::ptr::null());
    }
}

#[cfg(feature = "uv")]
impl NMSRaaSManager {
    #[instrument(level = "trace", skip(part_root))]
    async fn create_part_manager_for_mode(
        part_root: &VfsPath,
        render_type: &RenderMode,
    ) -> Result<PartsManager> {
        let path = part_root.join(render_type.to_string())?;

        Ok(PartsManager::new(&path)?)
    }
}

#[cfg(not(feature = "lazy_parts"))]
#[cfg(feature = "uv")]
impl NMSRaaSManager {
    pub(crate) fn get_manager(&self, render_type: &RenderMode) -> Result<Cow<PartsManager>> {
        self.part_managers
            .get(render_type)
            .map(Cow::Borrowed)
            .ok_or_else(|| MissingPartManager(render_type.clone()))
    }

    #[cfg(feature = "uv")]
    #[instrument(level = "trace", skip(part_root))]
    pub(crate) async fn new(part_root: impl AsRef<Path>) -> Result<NMSRaaSManager> {
        let part_root: VfsPath = PhysicalFS::new(part_root).into();
        let mut map = HashMap::with_capacity(RenderMode::COUNT);

        for render_type in RenderMode::iter() {
            let manager = Self::create_part_manager_for_mode(&part_root, &render_type)?;
            map.insert(render_type, manager);
        }

        Ok(NMSRaaSManager { part_managers: map })
    }
}

#[cfg(feature = "lazy_parts")]
impl NMSRaaSManager {
    pub(crate) fn get_manager(&self, render_type: &RenderMode) -> Result<Cow<PartsManager>> {
        let lazy_parts_dir = Self::get_lazy_parts_directory(&self.part_root)?;
        let part_path = Self::get_render_mode_part_manager_path(&lazy_parts_dir, render_type)?;

        if part_path.exists()? {
            let reader = BufReader::new(part_path.open_file()?);

            let start = std::time::Instant::now();
            let manager = bincode::deserialize_from(reader)?;
            debug!(
                "Deserialized part manager for {:?} in {:?}",
                render_type,
                start.elapsed()
            );

            Ok(Cow::Owned(manager))
        } else {
            Err(MissingPartManager(render_type.clone()))
        }
    }

    fn get_lazy_parts_directory(part_root: &VfsPath) -> Result<VfsPath> {
        Ok(part_root.join("lazy_parts")?)
    }

    fn get_render_mode_part_manager_path(
        lazy_parts_dir: &VfsPath,
        render_type: &RenderMode,
    ) -> Result<VfsPath> {
        Ok(lazy_parts_dir.join(render_type.to_string())?)
    }

    #[instrument(level = "trace", skip(part_root))]
    pub(crate) fn new(part_root: impl AsRef<Path>) -> Result<NMSRaaSManager> {
        let part_root = PhysicalFS::new(part_root).into();
        let lazy_parts_dir = Self::get_lazy_parts_directory(&part_root)?;

        // Yeet all the old parts we made just in case.
        // It's a one time action so it's fine™
        lazy_parts_dir.remove_dir_all()?;
        lazy_parts_dir.create_dir_all()?;

        let serialized_parts: Vec<_> = RenderMode::iter()
            .par_bridge()
            .map(|render_type| {
                let manager = Self::create_part_manager_for_mode(&part_root, &render_type);
                let serialized = manager.and_then(|manager| {
                    bincode::serialize(&manager).map_err(NMSRaaSError::BincodeError)
                });

                (render_type, serialized)
            })
            .collect();

        for (mode, serialized_part) in serialized_parts {
            let file = Self::get_render_mode_part_manager_path(&lazy_parts_dir, &mode)?;
            let mut writer = BufWriter::new(file.create_file()?);
            let data = serialized_part?;

            writer.write_all(data.as_slice())?;
        }

        Ok(NMSRaaSManager { part_root })
    }
}
