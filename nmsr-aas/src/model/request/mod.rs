use std::f32::consts;

use derive_more::Debug;
use enumset::{EnumSet, EnumSetType};
use nmsr_rendering::{high_level::{
    camera::Camera,
    pipeline::scene::{Size, SunInformation},
}, low_level::{Quat, EulerRot, Vec3}};
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
pub struct RenderRequestCameraSettings {
    pub yaw: Option<f32>,
    pub pitch: Option<f32>,
    pub roll: Option<f32>,

    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderRequest {
    pub mode: RenderRequestMode,
    pub entry: RenderRequestEntry,
    pub model: Option<RenderRequestEntryModel>,
    pub features: EnumSet<RenderRequestFeatures>,
    pub camera_settings: Option<RenderRequestCameraSettings>,
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
        camera_settings: Option<RenderRequestCameraSettings>,
    ) -> Self {
        RenderRequest {
            mode,
            entry,
            model,
            features: EnumSet::all().difference(excluded_features),
            camera_settings,
        }
    }
    
    pub(crate) fn get_camera(&self) -> Camera {
        let mut camera = self.mode.get_camera();
        
        if let Some(settings) = &self.camera_settings {
            if let Some(yaw) = settings.yaw {
                camera.set_yaw(yaw)
            }
            
            if let Some(pitch) = settings.pitch {
                camera.set_pitch(pitch)
            }
            
            if let Some(roll) = settings.roll {
                camera.set_roll(roll)
            }
        }
        
        camera
    }
    
    pub(crate) fn get_size(&self) -> Size {
        let mut size = self.mode.get_size();
        
        if let Some(settings) = &self.camera_settings {
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
        
        let rot_quat: Quat = Quat::from_euler(
            EulerRot::ZXY,
            camera.get_roll().to_radians(),
            -camera.get_pitch().to_radians(),
            camera.get_yaw().to_radians() - std::f32::consts::PI,
        ).into();
        
        let front_lighting = rot_quat.mul_vec3(Vec3::Z) * Vec3::new(1.0, 1.0, -1.0);
    
        return SunInformation::new(front_lighting, 1.0, 0.5);
    }
}
