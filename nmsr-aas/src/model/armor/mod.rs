pub mod manager;

use std::iter::repeat;

use nmsr_rendering::high_level::{
    model::{ArmorMaterial, PlayerArmorSlot},
    types::PlayerPartTextureType,
};
use strum::{Display, IntoStaticStr};

#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, IntoStaticStr, strum::EnumString, strum::EnumIter,
)]
pub enum VanillaMinecraftArmorMaterial {
    Chainmail,
    Diamond,
    Gold,
    Iron,
    Leather,
    Netherite,
    Turtle,
}

impl VanillaMinecraftArmorMaterial {
    fn layer_count(&self) -> usize {
        2
    }

    pub fn has_overlay(&self) -> bool {
        matches!(self, Self::Leather)
    }

    pub fn get_layer_name(&self, id: u32, is_overlay: bool) -> String {
        let name = self.to_string().to_lowercase();

        if is_overlay {
            format!("{name}_layer_{id}_overlay.png")
        } else {
            format!("{name}_layer_{id}.png")
        }
    }

    pub fn get_layer_names(&self) -> Vec<String> {
        let result = (0..self.layer_count()).map(|i| (i + 1).to_string());

        if self.has_overlay() {
            result
                .zip(repeat("overlay"))
                .map(|(layer, overlay)| format!("{}_{}", layer, overlay))
                .collect()
        } else {
            result.collect()
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum VanillaMinecraftArmorTrim {
    Coast,
    Dune,
    Eye,
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

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
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
    pub fn new(
        trim: VanillaMinecraftArmorTrim,
        material: VanillaMinecraftArmorTrimMaterial,
    ) -> Self {
        Self { trim, material }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VanillaMinecraftArmorMaterialData {
    pub material: VanillaMinecraftArmorMaterial,
    pub trim: Vec<VanillaMinecraftArmorTrimData>,
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

    pub fn new(material: VanillaMinecraftArmorMaterial) -> Self {
        Self {
            material,
            trim: Vec::new(),
        }
    }

    pub fn add_trim(
        &mut self,
        trim: VanillaMinecraftArmorTrim,
        material: VanillaMinecraftArmorTrimMaterial,
    ) -> &mut Self {
        self.trim
            .push(VanillaMinecraftArmorTrimData::new(trim, material));

        self
    }
}

impl ArmorMaterial for VanillaMinecraftArmorMaterialData {
    fn get_texture_type(slot: PlayerArmorSlot) -> Option<PlayerPartTextureType> {
        Some(if slot.is_leggings() {
            VanillaMinecraftArmorMaterialData::ARMOR_TEXTURE_TWO
        } else {
            VanillaMinecraftArmorMaterialData::ARMOR_TEXTURE_ONE
        })
    }
}

impl VanillaMinecraftArmorTrimPalette {
    fn get_trim_palette() -> [u32; 8] {
        [
            0xE0E0E0, 0xC0C0C0, 0xA0A0A0, 0x808080, 0x606060, 0x404040, 0x202020, 0x000000,
        ]
    }

    pub fn get_palette_colors(&self) -> [u32; 8] {
        // Generated using the nmsr-rendering-palette-extractor crate (in utils folder)
        // and is based on the vanilla textures in `assets\minecraft\textures\trims\`
        match self {
            Self::Amethyst => [
                0xC98FF3, 0x9A5CC6, 0x6C49AA, 0x523687, 0x422776, 0x361C6A, 0x240C53, 0x17063B,
            ],
            Self::Copper => [
                0xE3826C, 0xB4684D, 0x9A472C, 0x793C28, 0x6D3420, 0x5F2B18, 0x4C2010, 0x3D180B,
            ],
            Self::Diamond => [
                0xCBFFF5, 0x6EECD2, 0x2CBAA8, 0x1D969A, 0x0C788D, 0x076578, 0x04515F, 0x013C47,
            ],
            Self::DiamondDarker => [
                0x15B3A1, 0x0A9CA1, 0x048185, 0x027472, 0x065D5B, 0x095148, 0x034642, 0x04403E,
            ],
            Self::Emerald => [
                0x82F6AD, 0x0EC754, 0x11A036, 0x107B24, 0x0E7222, 0x09631B, 0x035013, 0x023D0E,
            ],
            Self::Gold => [
                0xFFFD90, 0xECD93F, 0xDEB12D, 0xB16712, 0xA0450A, 0x803503, 0x712D00, 0x572300,
            ],
            Self::GoldDarker => [
                0xC29C2A, 0xBA8327, 0xA35F14, 0x89470C, 0x803503, 0x712D00, 0x572300, 0x3E1B03,
            ],
            Self::Iron => [
                0xC5D2D4, 0xBFC9C8, 0x9DAAAA, 0x7B8989, 0x717D7D, 0x657070, 0x576363, 0x465151,
            ],
            Self::IronDarker => [
                0xA2B0B3, 0x8A9291, 0x6F7676, 0x576363, 0x3D4949, 0x313B3B, 0x2B3434, 0x1D2828,
            ],
            Self::Lapis => [
                0x416E97, 0x1C4D9C, 0x21497B, 0x123365, 0x112E63, 0x0C285A, 0x091E45, 0x051636,
            ],
            Self::Netherite => [
                0x5A575A, 0x443A3B, 0x312E31, 0x2F2727, 0x231E1E, 0x1A1616, 0x100C0C, 0x090707,
            ],
            Self::NetheriteDarker => [
                0x2E2829, 0x282425, 0x281D1D, 0x241A1A, 0x1F1717, 0x1D1313, 0x140F0F, 0x0B0909,
            ],
            Self::Quartz => [
                0xF2EFED, 0xF6EADF, 0xE3DBC4, 0xB6AD96, 0x908E80, 0x656156, 0x45433C, 0x2A2822,
            ],
            Self::Redstone => [
                0xE62008, 0xBD2008, 0x971607, 0x781101, 0x650B01, 0x520D06, 0x360803, 0x1D0502,
            ],
        }
    }

    fn get_darker_palette(&self) -> Option<Self> {
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
        &self,
        armor_material: VanillaMinecraftArmorMaterial,
    ) -> VanillaMinecraftArmorTrimPalette {
        if let (Ok(equivalent_armor_material), Some(darker_palette)) = (
            VanillaMinecraftArmorTrimMaterial::try_from(armor_material),
            VanillaMinecraftArmorTrimPalette::from(*self).get_darker_palette(),
        ) {
            if equivalent_armor_material == *self {
                return darker_palette;
            }
        }

        VanillaMinecraftArmorTrimPalette::from(*self)
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
            VanillaMinecraftArmorMaterial::Chainmail => Err(()),
            VanillaMinecraftArmorMaterial::Diamond => Ok(Self::Diamond),
            VanillaMinecraftArmorMaterial::Gold => Ok(Self::Gold),
            VanillaMinecraftArmorMaterial::Iron => Ok(Self::Iron),
            VanillaMinecraftArmorMaterial::Leather => Err(()),
            VanillaMinecraftArmorMaterial::Netherite => Ok(Self::Netherite),
            VanillaMinecraftArmorMaterial::Turtle => Err(()),
        }
    }
}
