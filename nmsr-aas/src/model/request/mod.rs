use derive_more::Debug;
use enumset::{EnumSet, EnumSetType};
use nmsr_rendering::high_level::{camera::{Camera, CameraRotation, ProjectionParameters}, pipeline::scene::SunInformation, types::PlayerBodyPartType};
use strum::{Display, EnumString, IntoEnumIterator};
use tracing::instrument;

use self::entry::{RenderRequestEntry, RenderRequestEntryModel};

pub mod cache;
pub mod entry;

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

#[derive(EnumString, Debug, PartialEq, Clone)]
#[strum(serialize_all = "snake_case")]
pub enum RenderRequestMode {
    Skin,
    #[strum(serialize = "fullbody", serialize = "full", serialize = "full_body")]
    FullBody,
    #[strum(serialize = "bodybust", serialize = "bust", serialize = "body_bust")]
    BodyBust,
    #[strum(serialize = "frontfull", serialize = "front_full")]
    FrontFull,
    #[strum(serialize = "frontbust", serialize = "front", serialize = "front_bust")]
    FrontBust,
    Face,
    Head,
    #[strum(serialize = "full_body_iso", serialize = "fullbodyiso")]
    FullBodyIso,
    #[strum(serialize = "head_iso", serialize = "headiso")]
    HeadIso,
}

impl RenderRequestMode {
    pub(crate) fn get_camera(&self) -> Camera {
        let look_at = [0.0, 16.5, 0.0].into();

        match self {
            Self::FullBody => Camera::new_orbital(
                look_at,
                45.0,
                CameraRotation {
                    yaw: 25.0,
                    pitch: 11.5,
                },
                ProjectionParameters::Perspective { fov: 45.0 },
                1.0,
            ),
            Self::FullBodyIso => Camera::new_orbital(
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
                Self::FullBody | Self::FullBodyIso => {
                    SunInformation::new([0.0, -1.0, 5.0].into(), 1.0, 0.7)
                }
                _ => SunInformation::new([0.0; 3].into(), 0.0, 1.0),
            }
        }
    }

    pub(crate) fn get_arm_rotation(&self) -> f32 {
        match self {
            Self::FullBody => 10.0,
            _ => 0.0,
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub(crate) fn get_body_parts(&self) -> Vec<PlayerBodyPartType> {
        match self {
            Self::FullBody | Self::FrontFull | Self::FullBodyIso => {
                PlayerBodyPartType::iter().collect()
            }
            Self::Head | Self::HeadIso | Self::Face => {
                vec![PlayerBodyPartType::Head, PlayerBodyPartType::HeadLayer]
            }
            Self::BodyBust | Self::FrontBust => {
                let excluded = vec![PlayerBodyPartType::LeftLeg, PlayerBodyPartType::RightLeg];

                PlayerBodyPartType::iter()
                    .filter(|m| excluded.contains(&m.get_non_layer_part()))
                    .collect()
            }
            Self::Skin => unreachable!()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderRequest {
    pub mode: RenderRequestMode,
    pub entry: RenderRequestEntry,
    pub model: Option<RenderRequestEntryModel>,
    pub features: EnumSet<RenderRequestFeatures>,
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
    ) -> Self {
        RenderRequest {
            mode,
            entry,
            model,
            features: EnumSet::all().difference(excluded_features),
        }
    }
}
