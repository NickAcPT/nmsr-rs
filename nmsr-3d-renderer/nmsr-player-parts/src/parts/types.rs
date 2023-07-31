#[derive(Copy, Clone)]
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

    fn get_non_layer_part(&self) -> Self {
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

pub enum PlayerPartTextureType {
    Skin,
    Cape,
    #[cfg(feature = "ears")]
    Ears,
}
