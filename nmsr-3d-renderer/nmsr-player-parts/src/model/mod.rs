use crate::{
    parts::uv::CubeFaceUvs,
    types::{PlayerBodyPartType, PlayerPartTextureType},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerModel {
    #[default]
    Steve,
    Alex,
}

impl PlayerModel {
    pub fn is_slim_arms(&self) -> bool {
        match self {
            PlayerModel::Steve => false,
            PlayerModel::Alex => true,
        }
    }
}

pub trait ArmorMaterial {
    fn get_texture_type(slot: PlayerArmorSlot) -> Option<PlayerPartTextureType> {
        None
    }

    fn get_texture_uvs(slot: PlayerArmorSlot) -> Option<CubeFaceUvs> {
        None
    }
}

impl ArmorMaterial for () {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerArmorSlot {
    Helmet,
    Chestplate,
    Leggings,
    Boots,
}

impl PlayerArmorSlot {
    pub fn get_offset(&self) -> f32 {
        if matches!(self, Self::Leggings) {
            0.5
        } else {
            1.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerArmorSlots<M: ArmorMaterial> {
    pub helmet: Option<M>,
    pub chestplate: Option<M>,
    pub leggings: Option<M>,
    pub boots: Option<M>,
}

impl<M: ArmorMaterial> Default for PlayerArmorSlots<M> {
    fn default() -> Self {
        Self {
            helmet: None,
            chestplate: None,
            leggings: None,
            boots: None,
        }
    }
}

impl<M: ArmorMaterial> PlayerArmorSlots<M> {
    pub fn get_armor_slot(&self, slot: PlayerArmorSlot) -> Option<&M> {
        match slot {
            PlayerArmorSlot::Helmet => self.helmet.as_ref(),
            PlayerArmorSlot::Chestplate => self.chestplate.as_ref(),
            PlayerArmorSlot::Leggings => self.leggings.as_ref(),
            PlayerArmorSlot::Boots => self.boots.as_ref(),
        }
    }

    pub fn get_armor_slots_for_part(&self, part: &PlayerBodyPartType) -> Vec<PlayerArmorSlot> {
        match part {
            PlayerBodyPartType::Head => vec![PlayerArmorSlot::Helmet],
            PlayerBodyPartType::Body => {
                vec![PlayerArmorSlot::Leggings, PlayerArmorSlot::Chestplate]
            }
            PlayerBodyPartType::LeftArm | PlayerBodyPartType::RightArm => {
                vec![PlayerArmorSlot::Chestplate]
            }
            PlayerBodyPartType::LeftLeg | PlayerBodyPartType::RightLeg => {
                vec![PlayerArmorSlot::Leggings]
            }

            PlayerBodyPartType::HeadLayer
            | PlayerBodyPartType::BodyLayer
            | PlayerBodyPartType::LeftArmLayer
            | PlayerBodyPartType::RightArmLayer
            | PlayerBodyPartType::LeftLegLayer
            | PlayerBodyPartType::RightLegLayer => {
                unreachable!("Layer parts should have been converted to non-layer parts")
            }
        }
    }
}
