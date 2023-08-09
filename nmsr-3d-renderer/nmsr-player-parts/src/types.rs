use strum::{Display, EnumIter, IntoStaticStr};

#[derive(Debug, Copy, Clone, EnumIter, Eq, PartialEq)]
pub enum PlayerBodyPartType {
    // Normal body parts
    Head,
    Body,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,

    // Layers
    HeadLayer,
    BodyLayer,
    LeftArmLayer,
    RightArmLayer,
    LeftLegLayer,
    RightLegLayer,
}

impl PlayerBodyPartType {
    pub fn is_layer(&self) -> bool {
        matches!(
            self,
            Self::HeadLayer
                | Self::BodyLayer
                | Self::LeftArmLayer
                | Self::RightArmLayer
                | Self::LeftLegLayer
                | Self::RightLegLayer
        )
    }

    pub fn get_non_layer_part(&self) -> Self {
        match self {
            Self::HeadLayer => Self::Head,
            Self::BodyLayer => Self::Body,
            Self::LeftArmLayer => Self::LeftArm,
            Self::RightArmLayer => Self::RightArm,
            Self::LeftLegLayer => Self::LeftLeg,
            Self::RightLegLayer => Self::RightLeg,
            _ => *self,
        }
    }
}

#[derive(Debug, Copy, Clone, Display, IntoStaticStr, PartialEq, Eq, Hash)]
pub enum PlayerPartTextureType {
    Skin,
    Cape,
    #[cfg(feature = "ears")]
    Ears,
}

impl PlayerPartTextureType {
    pub fn get_texture_size(&self) -> (u32, u32) {
        match self {
            Self::Skin => (64, 64),
            Self::Cape => (64, 32),
        }
    }
}
