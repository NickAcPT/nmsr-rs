use std::fmt::{Debug, Formatter};

#[cfg(feature = "ears")]
use ears_rs::{features::EarsFeatures, parser::EarsParser};
use image::buffer::ConvertBuffer;
use image::RgbaImage;

use crate::{errors::Result, parts::player_model::PlayerModel};

pub struct RenderingEntry {
    pub skin: RgbaImage,
    pub model: PlayerModel,
    pub render_shading: bool,
    pub render_layers: bool,
    #[cfg(feature = "ears")]
    pub ears_features: Option<EarsFeatures>,
}

impl Debug for RenderingEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderingEntry")
            .field("model", &self.model)
            .field("render_shading", &self.render_shading)
            .field("render_layers", &self.render_layers)
            .finish()
    }
}

impl RenderingEntry {
    pub fn process_skin(skin: RgbaImage) -> Result<RgbaImage> {
        // Make sure the skin is 64x64
        let mut skin = ears_rs::utils::upgrade_skin_if_needed(skin);

        #[cfg(feature = "ears")]
        {
            // If using Ears, process the erase sections specified in the Alfalfa data
            ears_rs::utils::process_erase_regions(&mut skin)?;
        }

        // Strip the alpha data from the skin
        ears_rs::utils::strip_alpha(&mut skin);

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
    pub fn new(
        skin: RgbaImage,
        slim_arms: bool,
        render_shading: bool,
        render_layers: bool,
    ) -> Result<RenderingEntry> {
        #[cfg(feature = "ears")]
        let ears_features = EarsParser::parse(&skin)?;

        let skin = RenderingEntry::process_skin(skin)?;

        Ok(RenderingEntry {
            skin: skin.convert(),
            model: match slim_arms {
                true => PlayerModel::Alex,
                false => PlayerModel::Steve,
            },
            render_shading,
            render_layers,
            #[cfg(feature = "ears")]
            ears_features,
        })
    }
}
