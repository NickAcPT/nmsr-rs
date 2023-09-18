use std::collections::HashMap;

#[cfg(feature = "ears")]
use ears_rs::features::EarsFeatures;
use itertools::Itertools;
use nmsr_rendering::high_level::{
    model::PlayerModel,
    parts::{
        part::Part,
        provider::{PartsProvider, PlayerPartProviderContext, PlayerPartsProvider},
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
    IntoEnumIterator,
};

pub struct ModelGenerationProject {
    providers: Vec<PlayerPartsProvider>,
    part_context: PlayerPartProviderContext<()>,
    textures: HashMap<PlayerPartTextureType, Vec<u8>>,
}

impl ModelGenerationProject {
    pub fn new(
        model: PlayerModel,
        layers: bool,
        textures: HashMap<PlayerPartTextureType, Vec<u8>>,
        #[cfg(feature = "ears")] ears_features: Option<EarsFeatures>,
    ) -> Self {
        let context = PlayerPartProviderContext::<()> {
            model,
            has_hat_layer: layers,
            has_layers: layers,
            has_cape: false,
            arm_rotation: 10.0,
            shadow_y_pos: None,
            shadow_is_square: false,
            armor_slots: None,
            #[cfg(feature = "ears")]
            ears_features,
        };

        Self {
            providers: [
                PlayerPartsProvider::Minecraft,
                #[cfg(feature = "ears")]
                PlayerPartsProvider::Ears,
            ]
            .to_vec(),
            part_context: context,
            textures,
        }
    }

    pub(crate) fn generate_parts(&self) -> Vec<Part> {
        PlayerBodyPartType::iter()
            .filter(|p| !(p.is_layer() || p.is_hat_layer()) || self.part_context.has_layers)
            .flat_map(|p| {
                self.providers
                    .iter()
                    .flat_map(move |provider| provider.get_parts(&self.part_context, p))
            })
            .collect_vec()
    }

    pub(crate) fn get_texture(&self, texture_type: PlayerPartTextureType) -> Option<&[u8]> {
        self.textures.get(&texture_type).map(|v| v.as_slice())
    }
}
