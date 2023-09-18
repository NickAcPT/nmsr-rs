use std::collections::HashMap;

#[cfg(feature = "ears")]
use ears_rs::features::EarsFeatures;
use glam::Vec3;
use itertools::Itertools;
use nmsr_rendering::high_level::{
    model::PlayerModel,
    parts::{
        part::{Part, PartAnchorInfo},
        provider::{PartsProvider, PlayerPartProviderContext, PlayerPartsProvider},
        uv::CubeFaceUvs,
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
                    .flat_map(Self::process_part)
            })
            .collect_vec()
    }

    fn process_part(part: Part) -> Vec<Part> {
        match part {
            Part::Cube { .. } => vec![part],
            Part::Quad {
                name,
                position,
                size,
                last_rotation,
                face_uv,
                normal,
                texture,
                ..
            } => {
                let mut result = vec![];
                let size = [size.x as u32, size.y as u32, size.z as u32];
                let uvs = CubeFaceUvs {
                    up: face_uv,
                    down: face_uv,
                    north: face_uv,
                    south: face_uv,
                    east: face_uv,
                    west: face_uv,
                };

                let mut cube = Part::new_cube(texture, [0; 3], size, uvs, name);
                cube.rotate(
                    Vec3::ZERO,
                    Some(PartAnchorInfo {
                        translation_anchor: position,
                        ..Default::default()
                    }),
                );
                
                if let Some((rot, mut anchor)) = last_rotation {
                    // Remove the translation anchor since the part is already translated.
                    anchor.translation_anchor = Vec3::ZERO;
                    
                    cube.rotate(rot, Some(anchor));
                }

                result.push(cube);
                result
            }
        }
    }

    pub(crate) fn get_texture(&self, texture_type: PlayerPartTextureType) -> Option<&[u8]> {
        self.textures.get(&texture_type).map(|v| v.as_slice())
    }
}
