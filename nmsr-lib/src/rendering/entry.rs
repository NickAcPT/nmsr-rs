use image::buffer::ConvertBuffer;
use image::RgbaImage;

use crate::{errors::NMSRError, errors::Result, parts::player_model::PlayerModel, uv::Rgba16Image};

pub struct RenderingEntry {
    pub skin: Rgba16Image,
    pub model: PlayerModel,
    pub render_shading: bool,
    pub render_layers: bool,
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

    /// Create a new rendering entry from a skin and a model
    ///
    /// # Arguments
    ///
    /// * `skin`: The skin to render
    /// * `slim_arms`: Whether the skin has slim arms or not
    /// * `render_shading`: Whether to render shading or not (this is internally called overlays)
    /// * `render_layers`: Whether to render the secondary skin layers or not
    pub fn new(skin: RgbaImage, slim_arms: bool, render_shading: bool, render_layers: bool) -> Result<RenderingEntry> {
        let skin = RenderingEntry::process_skin(skin)?;

        Ok(RenderingEntry {
            skin: skin.convert(),
            model: match slim_arms {
                true => PlayerModel::Alex,
                false => PlayerModel::Steve,
            },
            render_shading,
            render_layers,
        })
    }
}
