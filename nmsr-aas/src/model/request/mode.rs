use core::fmt::Debug;
use std::f32::consts::FRAC_1_SQRT_2;

use nmsr_rendering::high_level::{
    camera::{Camera, CameraRotation, ProjectionParameters},
    pipeline::scene::{Size, SunInformation},
    types::PlayerBodyPartType,
};
use strum::{EnumString, IntoEnumIterator};
use tracing::instrument;

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
    pub(crate) const fn is_isometric(&self) -> bool {
        matches!(
            self,
            RenderRequestMode::FullBodyIso | RenderRequestMode::HeadIso
        )
    }

    pub(crate) const fn is_bust(&self) -> bool {
        matches!(
            self,
            RenderRequestMode::BodyBust | RenderRequestMode::FrontBust
        )
    }

    pub(crate) const fn is_head(&self) -> bool {
        matches!(
            self,
            RenderRequestMode::Head | RenderRequestMode::Face | RenderRequestMode::HeadIso
        )
    }

    pub(crate) const fn is_square(&self) -> bool {
        self.is_bust() || self.is_head()
    }

    // [min_w, min_h, max_w, max_h]
    pub const fn size_constraints(&self) -> [u32; 4] {
        if self.is_square() {
            [
                Self::MIN_RENDER_WIDTH,
                Self::MIN_RENDER_WIDTH,
                Self::MAX_RENDER_WIDTH,
                Self::MAX_RENDER_WIDTH,
            ]
        } else {
            [
                Self::MIN_RENDER_WIDTH,
                Self::MIN_RENDER_HEIGHT,
                Self::MAX_RENDER_WIDTH,
                Self::MAX_RENDER_HEIGHT,
            ]
        }
    }

    pub fn validate_unit<T: PartialOrd + Debug>(
        unit: &'static str,
        value: Option<T>,
        min: T,
        max: T,
    ) -> Option<(&str, String)> {
        value
            .filter(|value| *value < min || *value > max)
            .map(|_| (unit, format!("between {:?} and {:?}", min, max)))
    }
}

impl RenderRequestMode {
    pub const DEFAULT_RENDER_WIDTH: u32 = 512;
    pub const DEFAULT_RENDER_HEIGHT: u32 = 869;

    pub const MAX_RENDER_WIDTH: u32 = Self::DEFAULT_RENDER_WIDTH * 2;
    pub const MAX_RENDER_HEIGHT: u32 = Self::DEFAULT_RENDER_HEIGHT * 2;

    pub const MIN_RENDER_WIDTH: u32 = Self::DEFAULT_RENDER_WIDTH / 32;
    pub const MIN_RENDER_HEIGHT: u32 = Self::DEFAULT_RENDER_HEIGHT / 32;

    pub(crate) fn get_size(&self) -> Size {
        if self.is_square() {
            return Size {
                width: Self::DEFAULT_RENDER_WIDTH,
                height: Self::DEFAULT_RENDER_WIDTH,
            };
        } else {
            return Size {
                width: Self::DEFAULT_RENDER_WIDTH,
                height: Self::DEFAULT_RENDER_HEIGHT,
            };
        }
    }

    pub(crate) fn get_camera(&self) -> Camera {
        let look_at = [0.0, 16.5, 0.0].into();

        match self {
            Self::FullBody => Camera::new_orbital(
                look_at,
                45.0,
                CameraRotation {
                    yaw: 20.0,
                    pitch: 10.0,
                    roll: 0.0,
                },
                ProjectionParameters::Perspective { fov: 45.0 },
                1.0,
            ),
            Self::FullBodyIso => Camera::new_orbital(
                look_at,
                45.0,
                CameraRotation {
                    yaw: 45.0,
                    pitch: FRAC_1_SQRT_2.atan().to_degrees(),
                    roll: 0.0,
                },
                ProjectionParameters::Orthographic { aspect: 17.0 },
                1.0,
            ),
            Self::HeadIso => Camera::new_orbital(
                look_at,
                45.0,
                CameraRotation {
                    yaw: 45.0,
                    pitch: FRAC_1_SQRT_2.atan().to_degrees(),
                    roll: 0.0,
                },
                ProjectionParameters::Orthographic { aspect: 17.0 },
                1.0,
            ),
            _ => unimplemented!("wgpu rendering is not yet implemented"),
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
            Self::Skin => unreachable!(),
        }
    }
}
