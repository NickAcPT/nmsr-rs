use crate::parts::part::Part;
use crate::player_model::PlayerModel;
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
#[derive(Copy, Clone)]
pub struct PlayerPartProviderContext {
    pub model: PlayerModel,
}

pub trait PartsProvider {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part>;
}

impl PartsProvider for PlayerPartsProvider {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part> {
        match self {
            Self::Minecraft => MinecraftPlayerPartsProvider.get_parts(context, body_part),
            _ => todo!(),
        }
    }
}
