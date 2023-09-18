use std::collections::HashMap;

#[cfg(feature = "ears")]
use ears_rs::features::EarsFeatures;
use glam::Vec3;
use itertools::Itertools;
use nmsr_rendering::high_level::{
    model::PlayerModel,
    parts::{
        part::Part,
        provider::{PartsProvider, PlayerPartProviderContext, PlayerPartsProvider},
        uv::{CubeFaceUvs, FaceUv},
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
            arm_rotation: 0.0,
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

    pub const DISCARD_FACE: FaceUv =
        FaceUv::new(u16::MAX, u16::MAX, u16::MAX, u16::MAX, false, false, 0);

    pub fn new_single_face(face: FaceUv, normal: Vec3) -> CubeFaceUvs {
        let mut faces = CubeFaceUvs {
            north: Self::DISCARD_FACE,
            south: Self::DISCARD_FACE,
            east: Self::DISCARD_FACE,
            west: Self::DISCARD_FACE,
            up: Self::DISCARD_FACE,
            down: Self::DISCARD_FACE,
        };
        
        if normal == Vec3::Y {
            faces.up = face;
        } else if normal == Vec3::NEG_Y {
            faces.down = face;
        } else if normal == Vec3::X {
            faces.east = face;
        } else if normal == Vec3::NEG_X {
            faces.west = face;
        } else if normal == Vec3::Z {
            faces.north = face;
        } else if normal == Vec3::NEG_Z {
            faces.south = face;
        } else {
            // Normal is not a cardinal direction, so we set all faces to the same value.
            faces.north = face;
            faces.south = face;
            faces.east = face;
            faces.west = face;
            faces.up = face;
            faces.down = face;
        }

        faces
    }

    pub(crate) fn get_texture(&self, texture_type: PlayerPartTextureType) -> Option<&[u8]> {
        self.textures.get(&texture_type).map(|v| v.as_slice())
    }
}
