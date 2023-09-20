use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use ears_rs::{alfalfa::AlfalfaDataKey, parser::EarsParser};
use glam::Vec2;
use image::RgbaImage;
use itertools::Itertools;
use nmsr_rendering::high_level::{
    model::PlayerModel,
    parts::{
        part::Part,
        provider::{PartsProvider, PlayerPartProviderContext, PlayerPartsProvider},
        uv::{FaceUv, FaceUvPoint},
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
    IntoEnumIterator,
};

use crate::blockbench::model::ModelFaceUv;

pub struct ModelGenerationProject {
    providers: Vec<PlayerPartsProvider>,
    part_context: PlayerPartProviderContext<()>,
    textures: HashMap<PlayerPartTextureType, RgbaImage>,
    max_resolution: Vec2,
}

impl ModelGenerationProject {
    pub fn new(model: PlayerModel, layers: bool) -> Self {
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
            ears_features: None,
        };

        Self {
            providers: [
                PlayerPartsProvider::Minecraft,
                #[cfg(feature = "ears")]
                PlayerPartsProvider::Ears,
            ]
            .to_vec(),
            part_context: context,
            textures: HashMap::new(),
            max_resolution: Vec2::ZERO,
        }
    }

    pub fn load_texture(
        &mut self,
        texture_type: PlayerPartTextureType,
        texture: &[u8],
    ) -> Result<()> {
        let texture = image::load_from_memory(texture)
            .context("Failed to load texture")?
            .into_rgba8();

        self.add_texture(texture_type, texture)
    }

    pub fn add_texture(
        &mut self,
        texture_type: PlayerPartTextureType,
        mut texture: RgbaImage,
    ) -> Result<()> {
        #[cfg(feature = "ears")]
        {
            use nmsr_rendering::high_level::parts::provider::ears::PlayerPartEarsTextureType;

            if texture_type == PlayerPartTextureType::Skin {
                if let Ok(Some(alfalfa)) = ears_rs::alfalfa::read_alfalfa(&texture) {
                    if let Some(wings) = alfalfa.get_data(AlfalfaDataKey::Wings) {
                        self.load_texture(PlayerPartEarsTextureType::Wings.into(), wings)?;
                    }

                    if let Some(cape) = alfalfa.get_data(AlfalfaDataKey::Cape) {
                        self.load_texture(PlayerPartEarsTextureType::Cape.into(), cape)?;
                    }
                }

                let features = EarsParser::parse(&texture)
                    .context(anyhow!("Failed to parse ears features from skin"))?;

                self.part_context.ears_features = features;

                ears_rs::utils::process_erase_regions(&mut texture)?;
            } else if texture_type == PlayerPartEarsTextureType::Cape.into()
            && texture_type.get_texture_size() != (texture.width(), texture.height())
            {
                texture = ears_rs::utils::convert_ears_cape_to_mojang_cape(texture);
            }
        }
        if texture_type == PlayerPartTextureType::Skin {
            ears_rs::utils::strip_alpha(&mut texture);
        } else if texture_type == PlayerPartTextureType::Cape {
            self.part_context.has_cape = true;
        }
        
        self.textures.insert(texture_type, texture);
        self.recompute_max_resolution();

        Ok(())
    }

    fn recompute_max_resolution(&mut self) {
        let max_resolution = self
            .textures
            .values()
            .map(|t| t.dimensions())
            .max_by_key(|(w, h)| w * h)
            .unwrap_or((0, 0));

        self.max_resolution = Vec2::new(max_resolution.0 as f32, max_resolution.1 as f32);
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

    pub(crate) fn get_texture(&self, texture_type: PlayerPartTextureType) -> Option<&RgbaImage> {
        self.textures.get(&texture_type)
    }

    pub(crate) fn max_resolution(&self) -> Vec2 {
        self.max_resolution
    }

    pub(crate) fn handle_face(&self, texture: PlayerPartTextureType, uv: FaceUv) -> ModelFaceUv {
        fn handle_coordinate(
            coordinate: FaceUvPoint,
            tex_resolution: Vec2,
            required_resolution: Vec2,
        ) -> Vec2 {
            let [mut x, mut y] = [coordinate.x as f32, coordinate.y as f32];

            let [tex_width, tex_height]: [f32; 2] = tex_resolution.into();
            let [required_x, required_y]: [f32; 2] = required_resolution.into();

            if tex_resolution != required_resolution {
                x = (x / tex_width) * required_x;
                y = (y / tex_height) * required_y;
            }

            [x, y].into()
        }

        let resolution = texture.get_texture_size();
        let resolution = Vec2::new(resolution.0 as f32, resolution.1 as f32);

        let mut uvs = [Vec2::ZERO; 4];

        for (index, coordinate) in [uv.top_left, uv.top_right, uv.bottom_right, uv.bottom_left]
            .into_iter()
            .enumerate()
        {
            uvs[index] = handle_coordinate(coordinate, resolution, self.max_resolution);
        }

        let [top_left, top_right, bottom_right, bottom_left] = uvs;

        ModelFaceUv {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    pub(crate) fn get_texture_id(&self, texture: PlayerPartTextureType) -> u32 {
        self.textures
            .keys()
            .sorted_by_key(|&&t| t)
            .enumerate()
            .find(|(_, &t)| t == texture)
            .map(|(i, _)| i as u32)
            .ok_or(anyhow!("Failed to find texture id for {:?}", texture))
            .unwrap()
    }
}
