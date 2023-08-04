use crate::parts::part::Part;
use crate::player_model::PlayerModel;
use crate::types::PlayerBodyPartType;

pub mod ears;
pub mod minecraft;

pub enum PlayerPartsProvider {
    Minecraft,
    #[cfg(feature = "ears")]
    Ears,
}

/// Context for player parts.
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
            Self::Minecraft => todo!(),
            _ => todo!(),
        }
    }
}
