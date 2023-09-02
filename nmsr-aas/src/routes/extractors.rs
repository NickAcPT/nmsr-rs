use crate::{
    error::{NMSRaaSError, RenderRequestError, Result},
    model::{
        armor::VanillaMinecraftArmorMaterialData,
        request::{
            entry::{RenderRequestEntry, RenderRequestEntryModel},
            RenderRequest, RenderRequestExtraSettings, RenderRequestFeatures, RenderRequestMode,
        },
    },
};
use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Path, Query},
    http::request::Parts,
    RequestPartsExt,
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
///  - `arms=<rotation>` or `arm=<rotation>`: set the rotation of the arms
///  - `dist=<distance>` or `distance=<distance>`: set the distance of the camera
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
struct RenderRequestQueryParams {
    #[serde_as(as = "Option<StringWithSeparator::<CommaSeparator, RenderRequestFeatures>>")]
    #[serde(alias = "no")]
    exclude: Option<EnumSet<RenderRequestFeatures>>,

    noshading: Option<String>,
    nolayers: Option<String>,

    #[serde(alias = "y")]
    yaw: Option<f32>,
    #[serde(alias = "p")]
    pitch: Option<f32>,
    #[serde(alias = "r")]
    roll: Option<f32>,

    #[serde(alias = "w")]
    width: Option<u32>,
    #[serde(alias = "h")]
    height: Option<u32>,

    #[serde_as(as = "Option<DisplayFromStr>")]
    model: Option<RenderRequestEntryModel>,
    alex: Option<String>,
    steve: Option<String>,

    process: Option<String>,

    #[serde(alias = "arm")]
    arms: Option<f32>,

    #[serde(alias = "d")]
    distance: Option<f32>,

    #[serde(alias = "xpos")]
    x_pos: Option<f32>,
    #[serde(alias = "ypos")]
    y_pos: Option<f32>,
    #[serde(alias = "zpos")]
    z_pos: Option<f32>,

    #[serde_as(as = "Option<StringWithSeparator::<CommaSeparator, f32>>")]
    pos: Option<Vec<f32>>,

    #[serde_as(as = "Option<TryFromInto<String>>")]
    helmet: Option<VanillaMinecraftArmorMaterialData>,
    #[serde_as(as = "Option<TryFromInto<String>>")]
    chestplate: Option<VanillaMinecraftArmorMaterialData>,
    #[serde_as(as = "Option<TryFromInto<String>>")]
    leggings: Option<VanillaMinecraftArmorMaterialData>,
    #[serde_as(as = "Option<TryFromInto<String>>")]
    boots: Option<VanillaMinecraftArmorMaterialData>,
}

impl RenderRequestQueryParams {
    fn get_excluded_features(&self) -> EnumSet<RenderRequestFeatures> {
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

    fn get_model(&self) -> Option<RenderRequestEntryModel> {
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

    fn validate(&mut self, mode: &RenderRequestMode) -> Result<()> {
        fn clamp(value: &mut Option<f32>, min: f32, max: f32) {
            let epsilon = 0.01;
            if let Some(value) = value {
                *value = value.clamp(min + epsilon, max - epsilon)
            }
        }

        let [min_w, min_h, max_w, max_h] = mode.size_constraints();

        RenderRequestMode::validate_unit("width", self.width, min_w, max_w)?;
        RenderRequestMode::validate_unit("height", self.height, min_h, max_h)?;

        RenderRequestMode::validate_unit("yaw", self.yaw, -180.0, 180.0)?;
        RenderRequestMode::validate_unit("pitch", self.pitch, -90.0, 90.0)?;
        RenderRequestMode::validate_unit("roll", self.roll, -180.0, 360.0)?;

        RenderRequestMode::validate_unit("arm", self.arms, 0.0, 180.0)?;

        //RenderRequestMode::validate_unit("distance", self.distance, -5.0, 30.0)?;

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
                "Switch to custom mode to make use of these.",
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

#[async_trait]
impl<S> FromRequestParts<S> for RenderRequest
where
    S: Send + Sync,
{
    type Rejection = NMSRaaSError;

    /// Extract a [`RenderRequest`] from the request parts.
    ///
    /// A [`RenderRequest`] contains an entry and its respective options.
    ///
    /// URLs have the following format:
    ///  - `/:model/:entry?options`
    ///
    /// The entry is in the URL path, and the options are in the query string.
    ///
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        let Path((mode_str, entry_str)): Path<(String, String)> = parts
            .extract_with_state::<Path<(String, String)>, S>(state)
            .await
            .map_err(RenderRequestError::from)?;

        let mode = RenderRequestMode::try_from(mode_str.as_str())
            .map_err(|_| RenderRequestError::InvalidRenderMode(mode_str))?;

        let entry = RenderRequestEntry::try_from(entry_str)?;

        let Query(mut query) = parts
            .extract_with_state::<Query<RenderRequestQueryParams>, S>(state)
            .await
            .map_err(RenderRequestError::from)?;

        query.validate(&mode)?;

        let excluded_features = query.get_excluded_features();

        let model = query.get_model();

        let extra_settings = Some(RenderRequestExtraSettings {
            width: query.width,
            height: query.height,

            yaw: query.yaw,
            pitch: query.pitch,
            roll: query.roll,

            arm_rotation: query.arms,
            distance: query.distance,

            x_pos: query.x_pos,
            y_pos: query.y_pos,
            z_pos: query.z_pos,
            
            helmet: query.helmet,
            chestplate: query.chestplate,
            leggings: query.leggings,
            boots: query.boots,
        });

        Ok(RenderRequest::new_from_excluded_features(
            mode,
            entry,
            model,
            excluded_features,
            extra_settings,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::{extract::State, routing::get, Router};
    use enumset::{enum_set, EnumSet};
    use hyper::{Body, Request};
    use tokio::sync::mpsc::Sender;
    use tower::ServiceExt;
    use uuid::uuid;

    use crate::model::request::{
        entry::{RenderRequestEntry, RenderRequestEntryModel},
        RenderRequest, RenderRequestFeatures, RenderRequestMode,
    };

    async fn render_request_from_url(url: &str) -> RenderRequest {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<RenderRequest>(1);

        let request = Request::builder()
            .uri(url)
            .body(Body::empty())
            .expect("Failed to build request");

        let app: Router = Router::new()
            .route(
                "/:mode/:entry",
                get(
                    |request: RenderRequest,
                     State(state): State<Sender<RenderRequest>>| async move {
                        state.send(request).await.unwrap();
                        ()
                    },
                ),
            )
            .with_state(tx);

        app.oneshot(request).await.expect("Failed to send request");

        rx.recv().await.expect("Failed to receive request")
    }

    #[tokio::test]
    async fn test_render_request_from_request_parts() {
        let entry =
            RenderRequestEntry::MojangPlayerUuid(uuid!("ad4569f3-7576-4376-a7c7-8e8cfcd9b832"));

        let expected = HashMap::from([
            (
                "http://localhost:8621/skin/ad4569f3-7576-4376-a7c7-8e8cfcd9b832",
                RenderRequest {
                    mode: RenderRequestMode::Skin,
                    entry: entry.clone(),
                    model: None,
                    features: EnumSet::ALL,
                    extra_settings: Some(Default::default())
                },
            ),
            (
                "http://localhost:8621/skin/ad4569f3-7576-4376-a7c7-8e8cfcd9b832?no=shadow",
                RenderRequest {
                    mode: RenderRequestMode::Skin,
                    entry: entry.clone(),
                    model: None,
                    features: EnumSet::all().difference(enum_set!(RenderRequestFeatures::Shadow)),
                    extra_settings: Some(Default::default())
                },
            ),
            (
                "http://localhost:8621/skin/ad4569f3-7576-4376-a7c7-8e8cfcd9b832?alex&noshading&nolayers",
                RenderRequest {
                    mode: RenderRequestMode::Skin,
                    entry: entry.clone(),
                    model: Some(RenderRequestEntryModel::Alex),
                    features: EnumSet::all().difference(enum_set!(RenderRequestFeatures::Shading | RenderRequestFeatures::BodyLayers | RenderRequestFeatures::HatLayer)),
                    extra_settings: Some(Default::default())
                },
            ),
            (
                "http://localhost:8621/fullbody/ad4569f3-7576-4376-a7c7-8e8cfcd9b832?nolayers&no=cape",
                RenderRequest {
                    mode: RenderRequestMode::FullBody,
                    entry: entry.clone(),
                    model: None,
                    features: EnumSet::all().difference(enum_set!(RenderRequestFeatures::BodyLayers | RenderRequestFeatures::HatLayer | RenderRequestFeatures::Cape)),
                    extra_settings: Some(Default::default())
                },
            ),
        ]);

        for (url, element) in expected {
            let result = render_request_from_url(url).await;

            assert_eq!(element, result, "Failed to extract for url: {}", url);
        }
    }
}
