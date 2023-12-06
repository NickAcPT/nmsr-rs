use crate::types::{PlayerBodyPartType, PlayerPartTextureType};

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
    pub fn get_slot_for_part(part: PlayerBodyPartType) -> Self {
        let part = part.get_non_layer_part();

        match part {
            PlayerBodyPartType::Head => Self::Helmet,
            PlayerBodyPartType::Body
            | PlayerBodyPartType::LeftArm
            | PlayerBodyPartType::RightArm => Self::Chestplate,
            PlayerBodyPartType::LeftLeg | PlayerBodyPartType::RightLeg => Self::Leggings,

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

    pub fn layer_id(&self) -> u32 {
        if self.is_leggings() {
            2
        } else {
            1
        }
    }

    pub fn is_leggings(&self) -> bool {
        matches!(self, Self::Leggings)
    }

    pub fn get_offset(&self) -> f32 {
        if self.is_leggings() {
            /* Dillation from the game */
            0.5 - /* Extra Leggings dillation */ 0.1
        } else {
            /* Dillation from the game */
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
    pub fn get_all_materials_in_slots(&self) -> Vec<(&M, PlayerArmorSlot)> {
        vec![
            self.helmet.as_ref().map(|a| (a, PlayerArmorSlot::Helmet)),
            self.chestplate
                .as_ref()
                .map(|a| (a, PlayerArmorSlot::Chestplate)),
            self.leggings
                .as_ref()
                .map(|a| (a, PlayerArmorSlot::Leggings)),
            self.boots.as_ref().map(|a| (a, PlayerArmorSlot::Boots)),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    pub fn get_armor_slot(&self, slot: PlayerArmorSlot) -> Option<&M> {
        match slot {
            PlayerArmorSlot::Helmet => self.helmet.as_ref(),
            PlayerArmorSlot::Chestplate => self.chestplate.as_ref(),
            PlayerArmorSlot::Leggings => self.leggings.as_ref(),
            PlayerArmorSlot::Boots => self.boots.as_ref(),
        }
    }

    pub fn get_parts_for_armor_slot(slot: PlayerArmorSlot) -> Vec<PlayerBodyPartType> {
        match slot {
            PlayerArmorSlot::Helmet => vec![PlayerBodyPartType::Head],
            PlayerArmorSlot::Chestplate => vec![
                PlayerBodyPartType::Body,
                PlayerBodyPartType::LeftArm,
                PlayerBodyPartType::RightArm,
            ],
            PlayerArmorSlot::Leggings => vec![
                PlayerBodyPartType::Body,
                PlayerBodyPartType::LeftLeg,
                PlayerBodyPartType::RightLeg,
            ],
            PlayerArmorSlot::Boots => {
                vec![PlayerBodyPartType::LeftLeg, PlayerBodyPartType::RightLeg]
            }
        }
    }

    pub fn get_armor_slots_for_part(part: &PlayerBodyPartType) -> Vec<PlayerArmorSlot> {
        match part {
            PlayerBodyPartType::Head => vec![PlayerArmorSlot::Helmet],
            PlayerBodyPartType::Body => {
                vec![PlayerArmorSlot::Leggings, PlayerArmorSlot::Chestplate]
            }
            PlayerBodyPartType::LeftArm | PlayerBodyPartType::RightArm => {
                vec![PlayerArmorSlot::Chestplate]
            }
            PlayerBodyPartType::LeftLeg | PlayerBodyPartType::RightLeg => {
                vec![PlayerArmorSlot::Leggings, PlayerArmorSlot::Boots]
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
