use strum::{EnumIter, IntoStaticStr};

#[derive(Debug, Copy, Clone, EnumIter, Eq, PartialEq, Hash, PartialOrd, Ord)]
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
            Self::BodyLayer
                | Self::LeftArmLayer
                | Self::RightArmLayer
                | Self::LeftLegLayer
                | Self::RightLegLayer
        )
    }

    pub fn is_hat_layer(&self) -> bool {
        matches!(self, Self::HeadLayer)
    }

    #[inline]
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
    
    pub fn get_layer_part(&self) -> Self {
        match self {
            Self::Head => Self::HeadLayer,
            Self::Body => Self::BodyLayer,
            Self::LeftArm => Self::LeftArmLayer,
            Self::RightArm => Self::RightArmLayer,
            Self::LeftLeg => Self::LeftLegLayer,
            Self::RightLeg => Self::RightLegLayer,
            _ => *self,
        }
    }

    pub fn is_arm(&self) -> bool {
        matches!(self.get_non_layer_part(), Self::LeftArm | Self::RightArm)
    }
    
    pub fn is_leg(&self) -> bool {
        matches!(self.get_non_layer_part(), Self::LeftLeg | Self::RightLeg)
    }
}

#[derive(Debug, Copy, Clone, IntoStaticStr, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PlayerPartTextureType {
    Shadow,
    Cape,
    Skin,
    Custom { key: &'static str, size: (u32, u32) },
}

impl std::fmt::Display for PlayerPartTextureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            PlayerPartTextureType::Shadow => f.pad("Shadow"),
            PlayerPartTextureType::Cape => f.pad("Cape"),
            PlayerPartTextureType::Skin => f.pad("Skin"),
            PlayerPartTextureType::Custom { key, .. } => f.pad(key)
        }
    }
}

impl PlayerPartTextureType {
    pub fn get_texture_size(&self) -> (u32, u32) {
        match self {
            Self::Skin => (64, 64),
            Self::Cape => (64, 32),
            Self::Custom { size, .. } => *size,
            Self::Shadow => (128, 128),
        }
    }

    pub fn is_shadow(&self) -> bool {
        matches!(self, Self::Shadow)
    }
}
