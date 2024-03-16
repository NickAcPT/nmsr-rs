pub mod manager;

use std::collections::VecDeque;

use nmsr_rendering::high_level::{
    model::{ArmorMaterial, PlayerArmorSlot},
    types::PlayerPartTextureType,
};
use strum::{Display, IntoEnumIterator, IntoStaticStr};

use crate::error::{ArmorManagerError, ArmorManagerResult};

#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, IntoStaticStr, strum::EnumString, strum::EnumIter,
)]
pub enum VanillaMinecraftArmorMaterial {
    Chainmail,
    Diamond,
    Gold,
    Iron,
    Leather(u64),
    Netherite,
    Turtle,
}

impl VanillaMinecraftArmorMaterial {
    const fn layer_count() -> usize {
        2
    }

    #[must_use]
    pub const fn has_overlay(&self) -> bool {
        matches!(self, Self::Leather(_))
    }

    #[must_use]
    pub fn get_layer_name(&self, id: u32, is_overlay: bool) -> String {
        let name = self.to_string().to_lowercase();

        if is_overlay {
            format!("{name}_layer_{id}_overlay.png")
        } else {
            format!("{name}_layer_{id}.png")
        }
    }

    #[must_use]
    pub fn get_layer_names(&self) -> Vec<String> {
        let result = (0..Self::layer_count()).map(|i| (i + 1).to_string());

        if self.has_overlay() {
            result
                .flat_map(|layer| vec![layer.clone(), format!("{}_overlay", &layer)])
                .collect()
        } else {
            result.collect()
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr, strum::EnumIter)]
pub enum VanillaMinecraftArmorTrim {
    Bolt,
    Coast,
    Dune,
    Eye,
    Flow,
    Host,
    Raiser,
    Rib,
    Sentry,
    Shaper,
    Silence,
    Snout,
    Spire,
    Tide,
    Vex,
    Ward,
    Wayfinder,
    Wild,
}

impl VanillaMinecraftArmorTrim {
    #[must_use]
    pub fn get_layer_names(&self) -> Vec<String> {
        let name = self.to_string().to_lowercase();

        vec![format!("{}.png", name), format!("{}_leggings.png", name)]
    }

    #[must_use]
    pub fn get_layer_name(&self, is_leggings: bool) -> String {
        let name = self.to_string().to_lowercase();

        if is_leggings {
            format!("{name}_leggings.png")
        } else {
            format!("{name}.png")
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, strum::EnumIter, strum::IntoStaticStr)]
pub enum VanillaMinecraftArmorTrimMaterial {
    Amethyst,
    Copper,
    Diamond,
    Emerald,
    Gold,
    Iron,
    Lapis,
    Netherite,
    Quartz,
    Redstone,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum VanillaMinecraftArmorTrimPalette {
    Amethyst,
    Copper,
    Diamond,
    DiamondDarker,
    Emerald,
    Gold,
    GoldDarker,
    Iron,
    IronDarker,
    Lapis,
    Netherite,
    NetheriteDarker,
    Quartz,
    Redstone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaMinecraftArmorTrimData {
    trim: VanillaMinecraftArmorTrim,
    material: VanillaMinecraftArmorTrimMaterial,
}

impl VanillaMinecraftArmorTrimData {
    #[must_use]
    pub const fn new(
        trim: VanillaMinecraftArmorTrim,
        material: VanillaMinecraftArmorTrimMaterial,
    ) -> Self {
        Self { trim, material }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VanillaMinecraftArmorMaterialData {
    pub material: VanillaMinecraftArmorMaterial,
    pub trims: Vec<VanillaMinecraftArmorTrimData>,
}

impl VanillaMinecraftArmorMaterialData {
    pub const ARMOR_TEXTURE_ONE: PlayerPartTextureType = PlayerPartTextureType::Custom {
        key: "armor_1",
        size: (64, 64),
    };

    pub const ARMOR_TEXTURE_TWO: PlayerPartTextureType = PlayerPartTextureType::Custom {
        key: "armor_2",
        size: (64, 64),
    };

    #[must_use]
    pub const fn new(material: VanillaMinecraftArmorMaterial) -> Self {
        Self {
            material,
            trims: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_trim(
        mut self,
        trim: VanillaMinecraftArmorTrim,
        material: VanillaMinecraftArmorTrimMaterial,
    ) -> Self {
        self.trims
            .push(VanillaMinecraftArmorTrimData::new(trim, material));

        self
    }
}

impl TryFrom<String> for VanillaMinecraftArmorMaterialData {
    type Error = ArmorManagerError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut split_values: VecDeque<_> = value.split('_').collect();
        let material: VanillaMinecraftArmorMaterial = if split_values.is_empty() {
            return Err(ArmorManagerError::EmptyArmorSlotError);
        } else {
            partial_match(split_values.pop_front().unwrap_or_default())?
        };

        let trims = if split_values.is_empty() {
            Vec::new()
        } else if split_values.len() % 2 != 0 {
            return Err(ArmorManagerError::InvalidTrimCountError(split_values.len()));
        } else {
            let values = split_values.make_contiguous();

            values
                .chunks_exact(2)
                .map(|chunk| {
                    let trim: VanillaMinecraftArmorTrim = partial_match(chunk[0])?;
                    let material: VanillaMinecraftArmorTrimMaterial = partial_match(chunk[1])?;

                    ArmorManagerResult::Ok(VanillaMinecraftArmorTrimData::new(trim, material))
                })
                .filter_map(std::result::Result::ok)
                .collect::<Vec<_>>()
        };

        Ok(Self { material, trims })
    }
}

fn partial_match<E>(value: &str) -> ArmorManagerResult<E>
where
    E: IntoEnumIterator + ToString,
{
    E::iter()
        .find(|x| {
            x.to_string()
                .to_lowercase()
                .starts_with(&value.to_lowercase())
        })
        .ok_or(ArmorManagerError::UnknownPartialArmorMaterialName(
            value.to_string(),
        ))
}

impl ArmorMaterial for VanillaMinecraftArmorMaterialData {
    fn get_texture_type(slot: PlayerArmorSlot) -> Option<PlayerPartTextureType> {
        Some(if slot.is_leggings() {
            Self::ARMOR_TEXTURE_TWO
        } else {
            Self::ARMOR_TEXTURE_ONE
        })
    }
}

impl VanillaMinecraftArmorTrimPalette {
    #[must_use]
    #[rustfmt::skip]
    const fn get_trim_palette() -> [[u8; 3]; 8] {
        [
            [0x00, 0x00, 0x00], [0x20, 0x20, 0x20], [0x40, 0x40, 0x40], [0x60, 0x60, 0x60], [0x80, 0x80, 0x80], [0xA0, 0xA0, 0xA0], [0xC0, 0xC0, 0xC0], [0xE0, 0xE0, 0xE0]
        ]
    }

    #[must_use]
    #[rustfmt::skip]
    pub const fn get_palette_colors(&self) -> [[u8; 3]; 8] {
        // Generated using the nmsr-rendering-palette-extractor crate (in utils folder)
        // and is based on the vanilla textures in `assets\minecraft\textures\trims\`
        match self {
            Self::Amethyst => [[0x17, 0x06, 0x3B], [0x24, 0x0C, 0x53], [0x36, 0x1C, 0x6A], [0x42, 0x27, 0x76], [0x52, 0x36, 0x87], [0x6C, 0x49, 0xAA], [0x9A, 0x5C, 0xC6], [0xC9, 0x8F, 0xF3]],
            Self::Copper => [[0x3D, 0x18, 0x0B], [0x4C, 0x20, 0x10], [0x5F, 0x2B, 0x18], [0x6D, 0x34, 0x20], [0x79, 0x3C, 0x28], [0x9A, 0x47, 0x2C], [0xB4, 0x68, 0x4D], [0xE3, 0x82, 0x6C]],
            Self::Diamond => [[0x01, 0x3C, 0x47], [0x04, 0x51, 0x5F], [0x07, 0x65, 0x78], [0x0C, 0x78, 0x8D], [0x1D, 0x96, 0x9A], [0x2C, 0xBA, 0xA8], [0x6E, 0xEC, 0xD2], [0xCB, 0xFF, 0xF5]],
            Self::DiamondDarker => [[0x02, 0x74, 0x72], [0x03, 0x46, 0x42], [0x04, 0x40, 0x3E], [0x04, 0x81, 0x85], [0x06, 0x5D, 0x5B], [0x09, 0x51, 0x48], [0x0A, 0x9C, 0xA1], [0x15, 0xB3, 0xA1]],
            Self::Emerald => [[0x02, 0x3D, 0x0E], [0x03, 0x50, 0x13], [0x09, 0x63, 0x1B], [0x0E, 0x72, 0x22], [0x0E, 0xC7, 0x54], [0x10, 0x7B, 0x24], [0x11, 0xA0, 0x36], [0x82, 0xF6, 0xAD]],
            Self::Gold => [[0x57, 0x23, 0x00], [0x71, 0x2D, 0x00], [0x80, 0x35, 0x03], [0xA0, 0x45, 0x0A], [0xB1, 0x67, 0x12], [0xDE, 0xB1, 0x2D], [0xEC, 0xD9, 0x3F], [0xFF, 0xFD, 0x90]],
            Self::GoldDarker => [[0x3E, 0x1B, 0x03], [0x57, 0x23, 0x00], [0x71, 0x2D, 0x00], [0x80, 0x35, 0x03], [0x89, 0x47, 0x0C], [0xA3, 0x5F, 0x14], [0xBA, 0x83, 0x27], [0xC2, 0x9C, 0x2A]],
            Self::Iron => [[0x46, 0x51, 0x51], [0x57, 0x63, 0x63], [0x65, 0x70, 0x70], [0x71, 0x7D, 0x7D], [0x7B, 0x89, 0x89], [0x9D, 0xAA, 0xAA], [0xBF, 0xC9, 0xC8], [0xC5, 0xD2, 0xD4]],
            Self::IronDarker => [[0x1D, 0x28, 0x28], [0x2B, 0x34, 0x34], [0x31, 0x3B, 0x3B], [0x3D, 0x49, 0x49], [0x57, 0x63, 0x63], [0x6F, 0x76, 0x76], [0x8A, 0x92, 0x91], [0xA2, 0xB0, 0xB3]],
            Self::Lapis => [[0x05, 0x16, 0x36], [0x09, 0x1E, 0x45], [0x0C, 0x28, 0x5A], [0x11, 0x2E, 0x63], [0x12, 0x33, 0x65], [0x1C, 0x4D, 0x9C], [0x21, 0x49, 0x7B], [0x41, 0x6E, 0x97]],
            Self::Netherite => [[0x09, 0x07, 0x07], [0x10, 0x0C, 0x0C], [0x1A, 0x16, 0x16], [0x23, 0x1E, 0x1E], [0x2F, 0x27, 0x27], [0x31, 0x2E, 0x31], [0x44, 0x3A, 0x3B], [0x5A, 0x57, 0x5A]],
            Self::NetheriteDarker => [[0x0B, 0x09, 0x09], [0x14, 0x0F, 0x0F], [0x1D, 0x13, 0x13], [0x1F, 0x17, 0x17], [0x24, 0x1A, 0x1A], [0x28, 0x1D, 0x1D], [0x28, 0x24, 0x25], [0x2E, 0x28, 0x29]],
            Self::Quartz => [[0x2A, 0x28, 0x22], [0x45, 0x43, 0x3C], [0x65, 0x61, 0x56], [0x90, 0x8E, 0x80], [0xB6, 0xAD, 0x96], [0xE3, 0xDB, 0xC4], [0xF2, 0xEF, 0xED], [0xF6, 0xEA, 0xDF]],
            Self::Redstone => [[0x1D, 0x05, 0x02], [0x36, 0x08, 0x03], [0x52, 0x0D, 0x06], [0x65, 0x0B, 0x01], [0x78, 0x11, 0x01], [0x97, 0x16, 0x07], [0xBD, 0x20, 0x08], [0xE6, 0x20, 0x08]],
        }
    }

    const fn get_darker_palette(self) -> Option<Self> {
        match self {
            Self::Diamond => Some(Self::DiamondDarker),
            Self::Gold => Some(Self::GoldDarker),
            Self::Iron => Some(Self::IronDarker),
            Self::Netherite => Some(Self::NetheriteDarker),
            _ => None,
        }
    }
}

impl VanillaMinecraftArmorTrimMaterial {
    fn get_palette_for_trim_armor_material(
        self,
        armor_material: VanillaMinecraftArmorMaterial,
    ) -> VanillaMinecraftArmorTrimPalette {
        if let (Ok(equivalent_armor_material), Some(darker_palette)) = (
            Self::try_from(armor_material),
            VanillaMinecraftArmorTrimPalette::from(self).get_darker_palette(),
        ) {
            if equivalent_armor_material == self {
                return darker_palette;
            }
        }

        VanillaMinecraftArmorTrimPalette::from(self)
    }
}

impl From<VanillaMinecraftArmorTrimMaterial> for VanillaMinecraftArmorTrimPalette {
    fn from(value: VanillaMinecraftArmorTrimMaterial) -> Self {
        match value {
            VanillaMinecraftArmorTrimMaterial::Amethyst => Self::Amethyst,
            VanillaMinecraftArmorTrimMaterial::Copper => Self::Copper,
            VanillaMinecraftArmorTrimMaterial::Diamond => Self::Diamond,
            VanillaMinecraftArmorTrimMaterial::Emerald => Self::Emerald,
            VanillaMinecraftArmorTrimMaterial::Gold => Self::Gold,
            VanillaMinecraftArmorTrimMaterial::Iron => Self::Iron,
            VanillaMinecraftArmorTrimMaterial::Lapis => Self::Lapis,
            VanillaMinecraftArmorTrimMaterial::Netherite => Self::Netherite,
            VanillaMinecraftArmorTrimMaterial::Quartz => Self::Quartz,
            VanillaMinecraftArmorTrimMaterial::Redstone => Self::Redstone,
        }
    }
}

impl TryFrom<VanillaMinecraftArmorMaterial> for VanillaMinecraftArmorTrimMaterial {
    type Error = ();

    fn try_from(value: VanillaMinecraftArmorMaterial) -> Result<Self, Self::Error> {
        match value {
            VanillaMinecraftArmorMaterial::Diamond => Ok(Self::Diamond),
            VanillaMinecraftArmorMaterial::Gold => Ok(Self::Gold),
            VanillaMinecraftArmorMaterial::Iron => Ok(Self::Iron),
            VanillaMinecraftArmorMaterial::Netherite => Ok(Self::Netherite),
            VanillaMinecraftArmorMaterial::Leather(_)
            | VanillaMinecraftArmorMaterial::Turtle
            | VanillaMinecraftArmorMaterial::Chainmail => Err(()),
        }
    }
}
