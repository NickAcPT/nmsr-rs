use crate::{types::{PlayerBodyPartType, PlayerPartTextureType}, parts::uv::CubeFaceUvs};

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

pub trait ArmorMaterial: Copy {
    fn get_offset(&self) -> Option<f32> {
        None
    }
    
    fn get_uvs(&self) -> Option<CubeFaceUvs> {
        None
    }
    
    fn get_texture_type(&self) -> Option<PlayerPartTextureType> {
        None
    }
}

impl ArmorMaterial for () {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerArmorSlots<M: ArmorMaterial> {
    pub helmet: M,
    pub chestplate: M,
    pub leggings: M,
    pub boots: M,
}

impl<M: ArmorMaterial> PlayerArmorSlots<M> {
    pub fn get_armor_slot_for_part(&self, part: &PlayerBodyPartType) -> Vec<&M> {
        match part {
            PlayerBodyPartType::Head => vec![&self.helmet],
            PlayerBodyPartType::Body => vec![&self.leggings, &self.chestplate],
            PlayerBodyPartType::LeftArm | PlayerBodyPartType::RightArm => vec![&self.chestplate],
            PlayerBodyPartType::LeftLeg | PlayerBodyPartType::RightLeg => vec![&self.leggings],

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
