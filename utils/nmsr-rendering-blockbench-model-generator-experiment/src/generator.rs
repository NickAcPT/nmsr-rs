use std::{
    collections::HashMap,
    io::{BufWriter, Cursor},
};

use glam::{Vec2, Vec3};
use image::RgbaImage;
use itertools::Itertools;
use nmsr_rendering::high_level::{
    model::{ArmorMaterial, PlayerModel},
    parts::{
        part::{Part, PartAnchorInfo},
        provider::{PartsProvider, PlayerPartProviderContext, PlayerPartsProvider},
        uv::{FaceUv, FaceUvPoint},
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
    IntoEnumIterator,
};

use crate::{
    blockbench::model::ModelFaceUv,
    error::{BlockbenchGeneratorError, Contextualizable, Result},
};

pub trait ModelProjectImageIO {
    fn read_png(&self, image: &[u8]) -> Result<RgbaImage>;
    fn write_png(&self, image: &RgbaImage) -> Result<Vec<u8>>;
}

pub struct DefaultImageIO;

impl ModelProjectImageIO for DefaultImageIO {
    fn read_png(&self, image: &[u8]) -> Result<RgbaImage> {
        let mut img = RgbaImage::new(1, 1);

        *img.get_pixel_mut(0, 0) = image::Rgba([255, 255, 255, 255]);
        
        return Ok(img);
    }

    fn write_png(&self, image: &RgbaImage) -> Result<Vec<u8>> {
        let mut bytes = Cursor::new(vec![]);

        {
            let mut writer = BufWriter::new(&mut bytes);
            image
                .write_to(&mut writer, image::ImageOutputFormat::Png)
                .context("Failed to write empty image to buffer")?;
        }

        Ok(bytes.into_inner())
    }
}

pub struct ModelGenerationProject<M: ArmorMaterial, I: ModelProjectImageIO> {
    providers: Vec<PlayerPartsProvider>,
    part_context: PlayerPartProviderContext<M>,
    textures: HashMap<PlayerPartTextureType, RgbaImage>,
    max_resolution: Vec2,
    image_io: I,
}

pub fn new_model_generator_without_part_context<I: ModelProjectImageIO>(
    model: PlayerModel,
    layers: bool,
    image_io: I,
) -> ModelGenerationProject<(), I> {
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

    ModelGenerationProject::new_with_part_context(image_io, context)
}

impl<M: ArmorMaterial, I: ModelProjectImageIO> ModelGenerationProject<M, I> {
    pub fn new_with_part_context(image_io: I, context: PlayerPartProviderContext<M>) -> Self {
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
            image_io,
        }
    }

    pub fn load_texture(
        &mut self,
        texture_type: PlayerPartTextureType,
        texture: &[u8],
        do_ears_processing: bool,
    ) -> Result<()> {
        let texture = self.image_io().read_png(texture)?;

        self.add_texture(texture_type, texture, do_ears_processing)
    }

    pub fn add_texture(
        &mut self,
        texture_type: PlayerPartTextureType,
        mut texture: RgbaImage,
        do_ears_processing: bool,
    ) -> Result<()> {
        if do_ears_processing {
            #[cfg(feature = "ears")]
            {
                use ears_rs::{alfalfa::AlfalfaDataKey, parser::EarsParser};
                use nmsr_rendering::high_level::parts::provider::ears::PlayerPartEarsTextureType;
                
                if texture_type == PlayerPartTextureType::Skin {
                    if let Ok(Some(alfalfa)) = ears_rs::alfalfa::read_alfalfa(&texture) {
                        if let Some(wings) = alfalfa.get_data(AlfalfaDataKey::Wings) {
                            self.load_texture(
                                PlayerPartEarsTextureType::Wings.into(),
                                wings,
                                do_ears_processing,
                            )?;
                        }

                        if let Some(cape) = alfalfa.get_data(AlfalfaDataKey::Cape) {
                            self.load_texture(
                                PlayerPartEarsTextureType::Cape.into(),
                                cape,
                                do_ears_processing,
                            )?;
                        }
                    }

                    let features = EarsParser::parse(&texture)?;

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
        
        let uvs = FaceUv { top_left: FaceUvPoint { x: 0, y: 0 }, top_right: FaceUvPoint { x: 0, y: 0 }, bottom_left: FaceUvPoint { x: 0, y: 0 }, bottom_right: FaceUvPoint { x: 0, y: 0 } };
        
        let first_quad = Part::new_quad(PlayerPartTextureType::Skin, [-8., 0., -8.], [16, 0, 8], uvs, Vec3::Z, None);
        let mut second_quad = Part::new_quad(PlayerPartTextureType::Skin, [-8., 0., -8.], [16, 0, 8], uvs, Vec3::Z, None);
        
        second_quad.rotate(Vec3::X * 90.0, Some(PartAnchorInfo::new_rotation_anchor_position(Vec3::new(0., 0., -8.0))));
        
        vec![
            first_quad,
            second_quad            
        ]
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

    pub(crate) fn get_texture_id(&self, texture: PlayerPartTextureType) -> Result<u32> {
        self.textures
            .keys()
            .sorted_by_key(|&&t| t)
            .enumerate()
            .find(|(_, &t)| t == texture)
            .map(|(i, _)| i as u32)
            .ok_or(BlockbenchGeneratorError::TextureNotFound(texture))
    }

    pub(crate) fn get_part_name(&self, name: Option<&str>, index: usize) -> String {
        name.to_owned()
            .map_or_else(|| format!("part-{index}"), |s| s.to_string())
    }

    pub(crate) fn image_io(&self) -> &I {
        &self.image_io
    }

    pub(crate) fn filter_textures(&mut self, keys: &[PlayerPartTextureType]) {
        self.textures.retain(|k, _| keys.contains(k));
    }
}
