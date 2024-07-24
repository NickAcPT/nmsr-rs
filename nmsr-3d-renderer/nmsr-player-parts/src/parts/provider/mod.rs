use self::minecraft::{perform_arm_part_rotation, MinecraftPlayerPartsProvider};
use crate::{model::{ArmorMaterial, PlayerArmorSlots, PlayerModel}};

#[cfg(feature = "part_tracker")]
use crate::parts::provider::minecraft::misc_part_set_origin;
use crate::types::PlayerBodyPartType;
use crate::{
    parts::{part::Part, uv::uv_from_pos_and_size},
    types::PlayerPartTextureType,
};
#[cfg(feature = "ears")]
use ears_rs::features::EarsFeatures;
use glam::Vec3;
use itertools::Itertools;
use strum::IntoEnumIterator;

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
    pub has_deadmau5_ears: bool,
    pub is_flipped_upside_down: bool,
    pub arm_rotation: f32,
    pub shadow_y_pos: Option<f32>,
    pub shadow_is_square: bool,
    pub armor_slots: Option<PlayerArmorSlots<M>>,
    #[cfg(feature = "ears")]
    pub ears_features: Option<EarsFeatures>,
}

impl<M> PlayerPartProviderContext<M>
where
    M: ArmorMaterial,
{
    pub fn get_all_parts(&self, providers: &[PlayerPartsProvider]) -> Vec<Part> {
        self.get_parts(providers, &PlayerBodyPartType::iter().collect_vec())
    }

    const MAX_PLAYER_HEIGHT: f32 = 32.0;

    pub fn get_parts(
        &self,
        providers: &[PlayerPartsProvider],
        body_parts: &[PlayerBodyPartType],
    ) -> Vec<Part> {
        let mut parts = providers
            .iter()
            .flat_map(|provider| {
                body_parts
                    .iter()
                    .flat_map(|part| provider.get_parts(self, *part))
            })
            .collect::<Vec<_>>();

        if self.is_flipped_upside_down {
            for part in &mut parts {
                part.rotate(Vec3::new(0.0, 0.0, 180.0), None);
                part.translate(Vec3::new(0.0, Self::MAX_PLAYER_HEIGHT, 0.0));
            }
        }

        if let Some(shadow_y_pos) = self.shadow_y_pos {
            let shadow = Part::new_quad(
                PlayerPartTextureType::Shadow,
                [-8.0, shadow_y_pos, -8.0],
                [16, 0, 16],
                uv_from_pos_and_size(0, 0, 128, 128),
                Vec3::Y,
                #[cfg(feature = "part_tracker")]
                Some("Shadow".to_string()),
            );
            // TODO: Expand shadow if there's armor on the feet

            parts.push(shadow);
        }

        return parts;
    }
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
        if (!context.has_layers && body_part.is_layer())
            || (!context.has_hat_layer && body_part.is_hat_layer())
        {
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

        for part in &mut parts {
            if body_part.is_arm() {
                perform_arm_part_rotation(
                    body_part.get_non_layer_part(),
                    part,
                    context.model.is_slim_arms(),
                    context.arm_rotation,
                );
            }
            
            #[cfg(feature = "part_tracker")]
            {
                misc_part_set_origin(body_part.get_non_layer_part(), part);
            }
        }

        parts
    }
}