use crate::parts::part::Part;
use crate::model::{PlayerModel, PlayerArmorSlots, ArmorMaterial};
use crate::types::PlayerBodyPartType;

use self::minecraft::MinecraftPlayerPartsProvider;

pub mod ears;
pub mod minecraft;

#[derive(Copy, Clone)]
pub enum PlayerPartsProvider {
    Minecraft,
    #[cfg(feature = "ears")]
    Ears,
}

/// Context for player parts.
#[derive(Copy, Clone, Default)]
pub struct PlayerPartProviderContext<M: = ()> where M: ArmorMaterial {
    pub model: PlayerModel,
    pub has_hat_layer: bool,
    pub has_layers: bool,
    pub has_cape: bool,
    pub arm_rotation: f32,
    pub shadow_y_pos: Option<f32>,
    pub shadow_is_square: bool,
    pub armor_slots: Option<PlayerArmorSlots<M>>,
}

pub trait PartsProvider<M: ArmorMaterial> {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext<M>,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part>;
}

impl<M: ArmorMaterial> PartsProvider<M> for PlayerPartsProvider {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext<M>,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part> {
        match self {
            Self::Minecraft => MinecraftPlayerPartsProvider::default().get_parts(context, body_part),
            #[cfg(feature = "ears")]
            Self::Ears => todo!(),
        }
    }
}
