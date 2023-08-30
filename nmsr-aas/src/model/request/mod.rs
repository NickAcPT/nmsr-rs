use derive_more::Debug;
use enumset::{EnumSet, EnumSetType};
use nmsr_rendering::{
    high_level::{
        camera::Camera,
        pipeline::scene::{Size, SunInformation},
    },
    low_level::{EulerRot, Quat, Vec3},
};
use strum::{Display, EnumString};

use self::entry::{RenderRequestEntry, RenderRequestEntryModel};

pub mod cache;
pub mod entry;
mod mode;

pub use mode::*;

#[derive(EnumSetType, EnumString, Debug, Display)]
#[strum(serialize_all = "snake_case")]
#[enumset(serialize_repr = "array")]
pub enum RenderRequestFeatures {
    #[strum(
        serialize = "overlay",
        serialize = "overlays",
        serialize = "body_layers",
        serialize = "layers"
    )]
    BodyLayers,
    #[strum(serialize = "helmet", serialize = "hat", serialize = "hat_layer")]
    HatLayer,
    Shadow,
    Shading,
    Cape,
    UnProcessedSkin,
    #[cfg(feature = "ears")]
    Ears,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RenderRequestExtraSettings {
    pub yaw: Option<f32>,
    pub pitch: Option<f32>,
    pub roll: Option<f32>,

    pub width: Option<u32>,
    pub height: Option<u32>,

    pub arm_rotation: Option<f32>,
    pub distance: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderRequest {
    pub mode: RenderRequestMode,
    pub entry: RenderRequestEntry,
    pub model: Option<RenderRequestEntryModel>,
    pub features: EnumSet<RenderRequestFeatures>,
    pub extra_settings: Option<RenderRequestExtraSettings>,
}

impl RenderRequest {
    /// Create a new RenderRequest from a render request entry and a set of features to exclude.
    ///
    /// # Arguments
    ///
    /// * `entry`: The entry used to create the RenderRequest.
    /// * `model`: The entry model used to create the RenderRequest.
    /// * `excluded_features`: The features to exclude from the RenderRequest.
    ///
    /// returns: The [RenderRequest] created from the entry and excluded features.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mode = RenderRequestMode::FullBody;
    /// let entry = RenderRequestEntry::PlayerUuid(uuid!("ad4569f3-7576-4376-a7c7-8e8cfcd9b832"));
    /// let excluded_features = enum_set!(RenderRequestFeatures::Shadow);
    /// let request = RenderRequest::new_from_excluded_features(mode, entry, None, excluded_features);
    /// ```
    pub fn new_from_excluded_features(
        mode: RenderRequestMode,
        entry: RenderRequestEntry,
        model: Option<RenderRequestEntryModel>,
        excluded_features: EnumSet<RenderRequestFeatures>,
        extra_settings: Option<RenderRequestExtraSettings>,
    ) -> Self {
        RenderRequest {
            mode,
            entry,
            model,
            features: EnumSet::all().difference(excluded_features),
            extra_settings,
        }
    }

    pub(crate) fn get_camera(&self) -> Camera {
        let mut camera = self.mode.get_camera();

        if let Some(settings) = &self.extra_settings {
            if let Some(yaw) = settings.yaw {
                camera.set_yaw(yaw)
            }

            if let Some(pitch) = settings.pitch {
                camera.set_pitch(pitch)
            }

            if let Some(roll) = settings.roll {
                camera.set_roll(roll)
            }

            if let Some(distance) = settings.distance {
                camera.set_distance(camera.get_distance() + distance)
            }
        }

        camera
    }

    pub(crate) fn get_size(&self) -> Size {
        let mut size = self.mode.get_viewport_size();

        if let Some(settings) = &self.extra_settings {
            if let Some(width) = settings.width {
                size.width = width;
            }

            if let Some(height) = settings.height {
                size.height = height;
            }
        }

        size
    }

    pub(crate) fn get_lighting(&self) -> SunInformation {
        if !self.features.contains(RenderRequestFeatures::Shading) {
            return SunInformation::new([0.0; 3].into(), 0.0, 1.0);
        }

        let camera = self.get_camera();

        let aligned_yaw = ((camera.get_yaw() + 180.0) / 90.0).floor() * 90.0;

        let rot_quat: Quat = Quat::from_euler(
            EulerRot::ZXY,
            camera.get_roll().to_radians(),
            0.0,
            aligned_yaw.to_radians(),
        )
        .into();

        let light = Vec3::new(0.0, 1.0, 6.21);
        let front_lighting = rot_quat.mul_vec3(light) * Vec3::new(1.0, 1.0, -1.0);

        return SunInformation::new(front_lighting, 1.0, 0.5);
    }

    pub(crate) fn get_arm_rotation(&self) -> f32 {
        if let Some(settings) = &self.extra_settings {
            if let Some(rotation) = settings.arm_rotation {
                return rotation;
            }
        }
        self.mode.get_arm_rotation()
    }
}
