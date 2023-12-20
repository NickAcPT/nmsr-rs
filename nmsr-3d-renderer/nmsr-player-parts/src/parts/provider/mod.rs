
use self::minecraft::{perform_arm_part_rotation, MinecraftPlayerPartsProvider};
use crate::model::{ArmorMaterial, PlayerArmorSlots, PlayerModel};
use crate::parts::part::Part;
use crate::types::PlayerBodyPartType;
#[cfg(feature = "ears")]
use ears_rs::features::EarsFeatures;

#[cfg(feature = "ears")]
pub mod ears;

#[cfg(all(feature = "ears"))]
use self::ears::EarsPlayerPartsProvider;

pub mod minecraft;

#[derive(Copy, Clone)]
pub enum PlayerPartsProvider {
    Minecraft,
    #[cfg(feature = "ears")]
    Ears,
}

/// Context for player parts.
#[derive(Debug, Copy, Clone, Default)]
pub struct PlayerPartProviderContext<M = ()>
where
    M: ArmorMaterial,
{
    pub model: PlayerModel,
    pub has_hat_layer: bool,
    pub has_layers: bool,
    pub has_cape: bool,
    pub arm_rotation: f32,
    pub shadow_y_pos: Option<f32>,
    pub shadow_is_square: bool,
    pub armor_slots: Option<PlayerArmorSlots<M>>,
    #[cfg(feature = "ears")]
    pub ears_features: Option<EarsFeatures>,
}

pub trait PartsProvider<M: ArmorMaterial> {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext<M>,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part>;
}

#[cfg(feature = "ears")]
pub(crate) static EARS_PLAYER_PARTS_PROVIDER: std::sync::OnceLock<EarsPlayerPartsProvider> =
    std::sync::OnceLock::new();

impl<M: ArmorMaterial> PartsProvider<M> for PlayerPartsProvider {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext<M>,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part> {
        if (!context.has_layers && body_part.is_layer()) || (!context.has_hat_layer && body_part.is_hat_layer()) {
            // Handle the case where we're asked to provide parts for a layer, but the context has explicitly stated that
            // it doesn't want layers.
            return vec![];
        }
        
        let mut parts = match self {
            Self::Minecraft => {
                MinecraftPlayerPartsProvider::default().get_parts(context, body_part)
            }
            #[cfg(feature = "ears")]
            Self::Ears => EARS_PLAYER_PARTS_PROVIDER
                .get_or_init(EarsPlayerPartsProvider::default)
                .get_parts(context, body_part),
        };

        if body_part.is_arm() {
            for part in &mut parts {
                perform_arm_part_rotation(
                    body_part.get_non_layer_part(),
                    part,
                    context.model.is_slim_arms(),
                    context.arm_rotation,
                );
            }
        }

        parts
    }
}
