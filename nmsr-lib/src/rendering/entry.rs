use crate::{errors::NMSRError, errors::Result, parts::player_model::PlayerModel, uv::Rgba16Image};
use image::buffer::ConvertBuffer;
use image::RgbaImage;

pub struct RenderingEntry {
    pub skin: Rgba16Image,
    pub model: PlayerModel,
}

impl RenderingEntry {
    pub fn process_skin(skin: RgbaImage) -> Result<RgbaImage> {
        // Make sure the skin is 64x64
        let mut skin = ears_rs::utils::legacy_upgrader::upgrade_skin_if_needed(skin)
            .ok_or(NMSRError::LegacySkinUpgradeError)?;

        // Strip the alpha data from the skin
        ears_rs::utils::alpha::strip_alpha(&mut skin);

        Ok(skin)
    }

    pub fn new(skin: RgbaImage, slim_arms: bool) -> Result<RenderingEntry> {
        let skin = RenderingEntry::process_skin(skin)?;

        Ok(RenderingEntry {
            skin: skin.convert(),
            model: match slim_arms {
                true => PlayerModel::Alex,
                false => PlayerModel::Steve,
            },
        })
    }
}
