use core::fmt::Debug;
use std::f32::consts::FRAC_1_SQRT_2;

use nmsr_rendering::high_level::{
    camera::{Camera, CameraRotation, ProjectionParameters},
    pipeline::scene::Size,
    types::PlayerBodyPartType,
};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};
use tracing::instrument;

use crate::error::{RenderRequestError, Result};

#[derive(EnumString, Debug, PartialEq, Eq, Clone, Copy, EnumIter, Display)]
#[strum(serialize_all = "snake_case")]
pub enum RenderRequestMode {
    #[strum(serialize = "skin", serialize = "texture")]
    Skin,
    #[strum(serialize = "cape", serialize = "cloak")]
    Cape,
    #[strum(serialize = "export", serialize = "bbmodel")]
    BlockbenchExport,
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
    Custom,
}

#[allow(dead_code)]
impl RenderRequestMode {
    pub(crate) const fn is_custom(self) -> bool {
        matches!(self, Self::Custom)
    }

    pub(crate) const fn is_full_body(self) -> bool {
        matches!(self, Self::FullBody | Self::FullBodyIso)
    }

    pub(crate) const fn is_isometric(self) -> bool {
        matches!(
            self,
            Self::FullBodyIso | Self::HeadIso | Self::FrontBust | Self::FrontFull | Self::Face
        )
    }

    pub(crate) const fn is_front(self) -> bool {
        matches!(self, Self::FrontBust | Self::FrontFull | Self::Face)
    }

    pub(crate) const fn is_bust(self) -> bool {
        matches!(self, Self::BodyBust | Self::FrontBust)
    }

    pub(crate) const fn is_arms_open(self) -> bool {
        matches!(
            self,
            Self::FullBody | Self::BodyBust | RenderRequestMode::BlockbenchExport
        )
    }

    pub(crate) const fn is_head_or_face(self) -> bool {
        matches!(self, Self::Head | Self::Face | Self::HeadIso)
    }

    pub(crate) const fn is_head(self) -> bool {
        matches!(self, Self::Head)
    }
    pub(crate) const fn is_head_iso(self) -> bool {
        matches!(self, Self::HeadIso)
    }

    pub(crate) const fn is_face(self) -> bool {
        matches!(self, Self::Face)
    }

    pub(crate) const fn is_square(self) -> bool {
        self.is_bust() || self.is_head_or_face()
    }

    pub(crate) const fn is_skin(self) -> bool {
        matches!(self, Self::Skin)
    }

    pub(crate) const fn is_cape(self) -> bool {
        matches!(self, Self::Cape)
    }

    pub(crate) const fn is_blockbench_export(self) -> bool {
        matches!(self, Self::BlockbenchExport)
    }

    pub(crate) const fn uses_rendering_pipeline(self) -> bool {
        !self.is_skin() && !self.is_cape() && !self.is_blockbench_export()
    }

    // [min_w, min_h, max_w, max_h]
    #[must_use]
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

    #[allow(unused_variables)]
    pub fn validate_unit<T: PartialOrd + Debug + Copy>(
        unit: &'static str,
        value: Option<T>,
        min: T,
        max: T,
    ) -> Result<()> {
        let check = value
            .filter(|value| *value < min || *value > max)
            .map(|_| (unit, format!("between {min:?} and {max:?}")));

        if let Some((unit, bounds)) = check {
            return Err(RenderRequestError::InvalidRenderSettingError(unit, bounds).into());
        }

        Ok(())
    }

    pub(crate) fn wrap_unit(value: Option<&mut f32>, min: f32, max: f32) -> Result<()> {
        if let Some(value) = value {
            *value = min + (*value - min + (max - min)) % (max - min)
        }
        
        return Ok(())
    }

    pub(crate) const fn get_base_render_mode(self) -> Option<Self> {
        match self {
            Self::BodyBust => Some(Self::FullBody),
            Self::FrontBust => Some(Self::FrontFull),
            _ => None,
        }
    }
}

impl RenderRequestMode {
    pub const DEFAULT_RENDER_WIDTH: u32 = 512;
    pub const DEFAULT_RENDER_HEIGHT: u32 = 869;

    pub const MAX_RENDER_WIDTH: u32 = Self::DEFAULT_RENDER_WIDTH * 2;
    pub const MAX_RENDER_HEIGHT: u32 = Self::DEFAULT_RENDER_HEIGHT * 2;

    pub const MIN_RENDER_WIDTH: u32 = Self::DEFAULT_RENDER_WIDTH / 32;
    pub const MIN_RENDER_HEIGHT: u32 = Self::DEFAULT_RENDER_HEIGHT / 32;

    pub(crate) const fn get_size(self) -> Size {
        if self.is_square() {
            Size {
                width: Self::DEFAULT_RENDER_WIDTH,
                height: Self::DEFAULT_RENDER_WIDTH,
            }
        } else {
            Size {
                width: Self::DEFAULT_RENDER_WIDTH,
                height: Self::DEFAULT_RENDER_HEIGHT,
            }
        }
    }

    pub(crate) fn get_camera(self) -> Camera {
        if let Some(base_mode) = self.get_base_render_mode() {
            let mut camera = base_mode.get_camera();
            camera.set_size(Some(base_mode.get_size()));

            return camera;
        }

        let mut look_at_y = 16.5;
        if self.is_head_or_face() {
            look_at_y += 11.5;
        }

        let mut distance = 45.0;
        if self.is_head_or_face() {
            distance -= 20.0;
        }
        if self.is_head() {
            distance -= 6.0;
        }

        let projection = if self.is_isometric() {
            let mut aspect = 17.0;

            if self.is_head_or_face() {
                aspect -= 9.5;
            }

            if self.is_face() {
                aspect -= 3.0;
            }

            ProjectionParameters::Orthographic { aspect }
        } else {
            ProjectionParameters::Perspective { fov: 45.0 }
        };

        let rotation = if self.is_front() || self.is_custom() {
            CameraRotation {
                yaw: 0.0,
                pitch: 0.0,
                roll: 0.0,
            }
        } else if self.is_isometric() {
            CameraRotation {
                yaw: 45.0,
                pitch: FRAC_1_SQRT_2.atan().to_degrees(),
                roll: 0.0,
            }
        } else if self.is_head() {
            CameraRotation {
                yaw: 25.0,
                pitch: 15.0,
                roll: 0.0,
            }
        } else {
            CameraRotation {
                yaw: 20.0,
                pitch: 10.0,
                roll: 0.0,
            }
        };

        let look_at = [0.0, look_at_y, 0.0].into();
        Camera::new_orbital(look_at, distance, rotation, projection, None)
    }

    pub(crate) const fn get_arm_rotation(self) -> f32 {
        if self.is_arms_open() {
            return 10.0;
        }
        0.0
    }

    #[instrument(level = "trace", skip(self))]
    pub(crate) fn get_body_parts(&self) -> Vec<PlayerBodyPartType> {
        match self {
            Self::Custom | Self::FullBody | Self::FrontFull | Self::FullBodyIso => {
                PlayerBodyPartType::iter().collect()
            }
            Self::Head | Self::HeadIso | Self::Face => {
                vec![PlayerBodyPartType::Head, PlayerBodyPartType::HeadLayer]
            }
            Self::BodyBust | Self::FrontBust => {
                let excluded = [PlayerBodyPartType::LeftLeg, PlayerBodyPartType::RightLeg];

                PlayerBodyPartType::iter()
                    .filter(|m| !excluded.contains(&m.get_non_layer_part()))
                    .collect()
            }
            Self::Cape | Self::Skin | Self::BlockbenchExport => unreachable!(),
        }
    }
}
