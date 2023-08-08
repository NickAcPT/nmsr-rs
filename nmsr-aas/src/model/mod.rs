pub(crate) mod resolver;

use enumset::{EnumSet, EnumSetType};
use strum::EnumString;

#[derive(Debug)]
pub(crate) enum RenderRequestEntry {
    PlayerUuid(uuid::Uuid),
    TextureHash(String),
    PlayerSkin(Vec<u8>)
}

#[derive(Debug, Default)]
pub(crate) enum RenderRequestEntryModel {
    #[default]
    Steve,
    Alex,
}

#[derive(EnumSetType, EnumString, Debug)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum RequestRenderFeatures {
    #[strum(serialize = "overlay", serialize = "overlays", serialize = "body_layers", serialize = "layers")]
    BodyLayers,
    #[strum(serialize = "helmet", serialize = "hat", serialize = "hat_layer")]
    HatLayer,
    Shadow,
    Cape,
    #[cfg(feature = "ears")]
    Ears,
}

#[derive(Debug)]
pub(crate) struct RenderRequest {
    pub(crate) entry: RenderRequestEntry,
    pub(crate) model: RenderRequestEntryModel,
    pub(crate) features: EnumSet<RequestRenderFeatures>,
}

impl RenderRequest {
    /// Create a new RenderRequest from a render request entry and a set of features to exclude.
    ///
    /// # Arguments
    ///
    /// * `entry`: The entry used to create the RenderRequest.
    /// * `model`: The entry model used to create the RenderRequest.
    /// * `excluded_features`: The features to exclude from the RenderRequest.
    ///
    /// returns: The [RenderRequest] created from the entry and excluded features.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let entry = RenderRequestEntry::PlayerUuid(uuid!("ad4569f3-7576-4376-a7c7-8e8cfcd9b832"));
    /// let excluded_features = enum_set!(RequestRenderFeatures::Shadow);
    /// let request = RenderRequest::new_from_excluded_features(entry, excluded_features);
    /// ```
    pub(crate) fn new_from_excluded_features(entry: RenderRequestEntry, model: RenderRequestEntryModel, excluded_features: EnumSet<RequestRenderFeatures>) -> Self {
        RenderRequest {
            entry,
            model,
            features: EnumSet::all().difference(excluded_features),
        }
    }
}
