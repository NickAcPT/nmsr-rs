use core::fmt::Debug;
use std::f32::consts::FRAC_1_SQRT_2;

use nmsr_rendering::high_level::{
    camera::{Camera, CameraPositionParameters, CameraRotation, ProjectionParameters},
    pipeline::scene::Size,
    types::PlayerBodyPartType,
};
use strum::{EnumString, IntoEnumIterator};
use tracing::instrument;

use crate::error::{RenderRequestError, Result};

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
    pub(crate) fn is_isometric(&self) -> bool {
        matches!(
            self,
            RenderRequestMode::FullBodyIso | RenderRequestMode::HeadIso
        )
    }

    pub(crate) fn is_bust(&self) -> bool {
        matches!(
            self,
            RenderRequestMode::BodyBust | RenderRequestMode::FrontBust
        )
    }

    pub(crate) fn is_arms_open(&self) -> bool {
        matches!(
            self,
            RenderRequestMode::FullBody | RenderRequestMode::BodyBust
        )
    }

    pub(crate) fn is_head(&self) -> bool {
        matches!(
            self,
            RenderRequestMode::Head | RenderRequestMode::Face | RenderRequestMode::HeadIso
        )
    }

    pub(crate) fn is_square(&self) -> bool {
        self.is_bust() || self.is_head()
    }

    // [min_w, min_h, max_w, max_h]
    pub fn size_constraints(&self) -> [u32; 4] {
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

    #[allow(unused_variables)]
    pub fn validate_unit<T: PartialOrd + Debug>(
        unit: &'static str,
        value: Option<T>,
        min: T,
        max: T,
    ) -> Result<()> {
        let check = value
            .filter(|value| *value < min || *value > max)
            .map(|_| (unit, format!("between {:?} and {:?}", min, max)));
        
        if let Some((unit, bounds)) = check {
            return Err(RenderRequestError::InvalidRenderSetting(unit, bounds).into());
        }
        
        return Ok(());
    }
}

impl RenderRequestMode {
    pub const DEFAULT_RENDER_WIDTH: u32 = 512;
    pub const DEFAULT_RENDER_HEIGHT: u32 = 869;

    pub const MAX_RENDER_WIDTH: u32 = Self::DEFAULT_RENDER_WIDTH * 2;
    pub const MAX_RENDER_HEIGHT: u32 = Self::DEFAULT_RENDER_HEIGHT * 2;

    pub const RENDER_ASPECT_RATIO: f32 =
        Self::DEFAULT_RENDER_WIDTH as f32 / Self::DEFAULT_RENDER_HEIGHT as f32;

    pub const MIN_RENDER_WIDTH: u32 = Self::DEFAULT_RENDER_WIDTH / 32;
    pub const MIN_RENDER_HEIGHT: u32 = Self::DEFAULT_RENDER_HEIGHT / 32;

    pub(crate) fn get_viewport_size(&self) -> Size {
        if self.is_square() {
            return Size {
                width: Self::DEFAULT_RENDER_WIDTH,
                height: Self::DEFAULT_RENDER_WIDTH,
            };
        } else {
            return self.get_size();
        }
    }
    
    pub(crate) fn get_size(&self) -> Size {
        return Size {
            width: Self::DEFAULT_RENDER_WIDTH,
            height: Self::DEFAULT_RENDER_HEIGHT,
        };
    }

    pub(crate) fn get_camera(&self) -> Camera {
        let look_at_y = if self.is_head() {
            28.5
        } else {
            16.5
        };

        let look_at = [0.0, look_at_y, 0.0].into();
        let distance = if self.is_head() {
            25.0
        } else {
            45.0
        };

        let projection = if self.is_isometric() {
            let aspect = if self.is_head() { 7.5 } else { 17.0 };

            ProjectionParameters::Orthographic { aspect }
        } else {
            ProjectionParameters::Perspective { fov: 45.0 }
        };

        let rotation = if self.is_isometric() {
            CameraRotation {
                yaw: 45.0,
                pitch: FRAC_1_SQRT_2.atan().to_degrees(),
                roll: 0.0,
            }
        } else {
            CameraRotation {
                yaw: 20.0,
                pitch: 10.0,
                roll: 0.0,
            }
        };

        Camera::new_orbital(look_at, distance, rotation, projection, Some(RenderRequestMode::FullBody.get_size()))
    }

    pub(crate) fn get_arm_rotation(&self) -> f32 {
        if self.is_arms_open() {
            return 10.0;
        }
        return 0.0;
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
                    .filter(|m| !excluded.contains(&m.get_non_layer_part()))
                    .collect()
            }
            Self::Skin => unreachable!(),
        }
    }
}
