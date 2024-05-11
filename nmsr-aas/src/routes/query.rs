use crate::{
    error::{RenderRequestError, Result},
    model::{
        armor::VanillaMinecraftArmorMaterialData,
        request::{entry::RenderRequestEntryModel, RenderRequestFeatures, RenderRequestMode},
    },
};
use enumset::EnumSet;
use serde::Deserialize;
use serde_with::TryFromInto;
use serde_with::{formats::CommaSeparator, serde_as, DisplayFromStr, StringWithSeparator};

///  The options are:
///  - `?exclude=<features>` or `?no=<features>`: exclude a feature from the entry (comma-separated, or multiple query strings)
///
///  - `?noshading`: disable shading of the entry [compatibility with old URLs]
///  - `?nolayers`: disable layers of the entry [compatibility with old URLs]
///
///  - `?y=<yaw>` or `?yaw=<yaw>`: set the yaw of the camera
///  - `?p=<pitch>` or `?pitch=<pitch>`: set the pitch of the camera
///  - `?r=<roll>` or `?roll=<roll>`: set the roll of the camera
///
///  - `?w=<width>` or `?width=<width>`: set the width of the image
///  - `?h=<height>` or `?height=<height>`: set the height of the image
///  - `?model=<steve|alex|wide|slim>`: set the model of the entry
///  - `?alex`: set the model of the entry to alex [compatibility with old URLs]
///  - `?steve`: set the model of the entry to steve [compatibility with old URLs]
///  - `?process`: process the skin (upgrade skin to 1.8 format, strip alpha from the body regions, apply erase regions if Ears feature is enabled)
///  
///  - `?arms=<rotation>` or `arm=<rotation>`: set the rotation of the arms
///  - `?dist=<distance>` or `distance=<distance>`: set the distance of the camera
///
///  - `xpos=<x>` or `x_pos=<x>`: set the x position of the camera (requires using Custom mode)
///  - `ypos=<y>` or `y_pos=<y>`: set the y position of the camera (requires using Custom mode)
///  - `zpos=<z>` or `z_pos=<z>`: set the z position of the camera (requires using Custom mode)
///
///  - `pos=<x>,<y>,<z>`: set the position of the camera (requires using Custom mode)
///
///  - `?helmet=<helmet>`: set the helmet of the entry
///  - `?chestplate=<chestplate>`: set the chestplate of the entry
///  - `?leggings=<leggings>`: set the leggings of the entry
///  - `?boots=<boots>`: set the boots of the entry
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct RenderRequestQueryParams {
    #[serde_as(as = "Option<StringWithSeparator::<CommaSeparator, RenderRequestFeatures>>")]
    #[serde(alias = "no")]
    pub exclude: Option<EnumSet<RenderRequestFeatures>>,

    pub noshading: Option<String>,
    pub nolayers: Option<String>,

    #[serde(alias = "y")]
    pub yaw: Option<f32>,
    #[serde(alias = "p")]
    pub pitch: Option<f32>,
    #[serde(alias = "r")]
    pub roll: Option<f32>,

    #[serde(alias = "w")]
    pub width: Option<u32>,
    #[serde(alias = "h")]
    pub height: Option<u32>,

    #[serde_as(as = "Option<DisplayFromStr>")]
    pub model: Option<RenderRequestEntryModel>,
    pub alex: Option<String>,
    pub steve: Option<String>,

    pub process: Option<String>,

    #[serde(alias = "arm")]
    pub arms: Option<f32>,

    #[serde(alias = "d")]
    pub distance: Option<f32>,

    #[serde(alias = "xpos")]
    pub x_pos: Option<f32>,
    #[serde(alias = "ypos")]
    pub y_pos: Option<f32>,
    #[serde(alias = "zpos")]
    pub z_pos: Option<f32>,

    #[serde_as(as = "Option<StringWithSeparator::<CommaSeparator, f32>>")]
    pub pos: Option<Vec<f32>>,

    #[serde_as(as = "Option<TryFromInto<String>>")]
    pub helmet: Option<VanillaMinecraftArmorMaterialData>,
    #[serde_as(as = "Option<TryFromInto<String>>")]
    pub chestplate: Option<VanillaMinecraftArmorMaterialData>,
    #[serde_as(as = "Option<TryFromInto<String>>")]
    pub leggings: Option<VanillaMinecraftArmorMaterialData>,
    #[serde_as(as = "Option<TryFromInto<String>>")]
    pub boots: Option<VanillaMinecraftArmorMaterialData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenderRequestMultipartParams {
    #[serde(flatten)]
    pub query: RenderRequestQueryParams,
    #[serde(alias = "texture")]
    pub skin: Vec<u8>,
    pub cape: Option<Vec<u8>>,
}

impl RenderRequestQueryParams {
    pub fn get_excluded_features(&self) -> EnumSet<RenderRequestFeatures> {
        let mut excluded = self.exclude.unwrap_or(EnumSet::EMPTY);

        if self.nolayers.is_some() {
            excluded |= RenderRequestFeatures::BodyLayers | RenderRequestFeatures::HatLayer;
        }

        if self.noshading.is_some() {
            excluded |= RenderRequestFeatures::Shading;
        }

        if self.process.is_some() {
            excluded |= RenderRequestFeatures::UnProcessedSkin;
        }

        excluded
    }

    pub fn get_model(&self) -> Option<RenderRequestEntryModel> {
        let steve = self
            .steve
            .as_ref()
            .and(Some(RenderRequestEntryModel::Steve));
        let alex = self.alex.as_ref().and(Some(RenderRequestEntryModel::Alex));
        let model = self.model;

        // Extract the model in the following order:
        // - First, check if the user specified that they wanted steve or alex (for compatibility with old URLs)
        // - Then, check if the user specified a model
        // Priority: Alex > Steve > Model
        alex.or(steve).or(model)
    }

    pub fn validate(&mut self, mode: RenderRequestMode) -> Result<()> {
        fn clamp(value: &mut Option<f32>, min: f32, max: f32) {
            let epsilon = 0.01;
            if let Some(value) = value {
                *value = value.clamp(min + epsilon, max - epsilon);
            }
        }

        let [min_w, min_h, max_w, max_h] = mode.size_constraints();

        RenderRequestMode::validate_unit("width", self.width, min_w, max_w)?;
        RenderRequestMode::validate_unit("height", self.height, min_h, max_h)?;

        RenderRequestMode::wrap_unit(self.yaw.as_mut(), -180.0, 180.0)?;
        RenderRequestMode::wrap_unit(self.pitch.as_mut(), -90.0, 90.0)?;
        RenderRequestMode::wrap_unit(self.roll.as_mut(), -180.0, 360.0)?;

        RenderRequestMode::validate_unit("arm", self.arms, 0.0, 180.0)?;

        RenderRequestMode::validate_unit("distance", self.distance, -15.0, 50.0)?;

        // Clamp yaw, pitch, roll so that there is no weirdness with the camera
        clamp(&mut self.yaw, -180.0, 180.0);
        clamp(&mut self.pitch, -90.0, 90.0);
        clamp(&mut self.roll, -180.0, 360.0);

        if !mode.is_custom() && self.width.is_some() && self.height.is_some() {
            return Err(RenderRequestError::InvalidModeSettingSpecifiedError(
                "both width and height settings",
                "Pick one or the other to use as a constraint based on aspect-ratio or switch to custom mode.",
            ).into());
        }

        let has_positions =
            self.x_pos.or(self.y_pos).or(self.z_pos).is_some() || self.pos.is_some();

        if !mode.is_custom() && (has_positions) {
            return Err(RenderRequestError::InvalidModeSettingSpecifiedError(
                "camera positions",
                "To fix this, switch to custom mode to make use of these. An automatic switch isn't possible due to how the different modes are handled.",
            )
            .into());
        }

        if let Some(pos) = &self.pos {
            if pos.len() != 3 {
                return Err(RenderRequestError::InvalidRenderSettingError(
                    "camera position (pos parameter)",
                    "3 valid numbers separated by commas".to_string(),
                )
                .into());
            }

            self.x_pos.replace(pos[0]);
            self.y_pos.replace(pos[1]);
            self.z_pos.replace(pos[2]);
        }

        RenderRequestMode::validate_unit("xpos", self.x_pos, -50.0, 50.0)?;
        RenderRequestMode::validate_unit("ypos", self.y_pos, -50.0, 50.0)?;
        RenderRequestMode::validate_unit("zpos", self.z_pos, -50.0, 50.0)?;

        Ok(())
    }
}
