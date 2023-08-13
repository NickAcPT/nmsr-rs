pub(crate) mod caching_v2;
pub(crate) mod resolver;

use enumset::{EnumSet, EnumSetType};
use strum::EnumString;

#[cfg(feature = "wgpu")]
use nmsr_rendering::high_level::player_model::PlayerModel;

use crate::utils::Result;
use crate::utils::errors::NMSRaaSError;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum RenderRequestEntry {
    PlayerUuid(uuid::Uuid),
    TextureHash(String),
    PlayerSkin(Vec<u8>),
}

impl TryFrom<String> for RenderRequestEntry {
    type Error = NMSRaaSError;

    fn try_from(value: String) -> Result<RenderRequestEntry> {
        if value.len() == 32 || value.len() == 36 {
            let uuid = uuid::Uuid::parse_str(&value).map_err(NMSRaaSError::InvalidUUID)?;
            let uuid_version = uuid.get_version_num();

            if uuid_version == 4 {
                Ok(RenderRequestEntry::PlayerUuid(uuid))
            } else {
                Err(NMSRaaSError::InvalidPlayerUuidRequest(value, uuid_version))
            }
        } else if value.len() > 36 {
            Ok(RenderRequestEntry::TextureHash(value))
        } else {
            Err(NMSRaaSError::InvalidPlayerRequest(value))
        }
    }
}

impl std::fmt::Debug for RenderRequestEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlayerUuid(arg0) => f.debug_tuple("PlayerUuid").field(arg0).finish(),
            Self::TextureHash(arg0) => f.debug_tuple("TextureHash").field(arg0).finish(),
            Self::PlayerSkin(_) => f.debug_tuple("PlayerSkin").finish(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, strum::FromRepr, strum::EnumCount)]
pub(crate) enum RenderRequestEntryModel {
    #[default]
    Steve,
    Alex,
}

#[cfg(feature = "wgpu")]
impl From<RenderRequestEntryModel> for PlayerModel {
    fn from(value: RenderRequestEntryModel) -> Self {
        match value {
            RenderRequestEntryModel::Steve => PlayerModel::Steve,
            RenderRequestEntryModel::Alex => PlayerModel::Alex,
        }
    }
}

#[allow(non_snake_case)]
#[derive(EnumSetType, EnumString, Debug)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum RequestRenderFeatures {
    #[strum(
        serialize = "overlay",
        serialize = "overlays",
        serialize = "body_layers",
        serialize = "layers"
    )]
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
    pub(crate) model: Option<RenderRequestEntryModel>,
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
    /// let request = RenderRequest::new_from_excluded_features(entry, None, excluded_features);
    /// ```
    pub(crate) fn new_from_excluded_features(
        entry: RenderRequestEntry,
        model: Option<RenderRequestEntryModel>,
        excluded_features: EnumSet<RequestRenderFeatures>,
    ) -> Self {
        RenderRequest {
            entry,
            model,
            features: EnumSet::all().difference(excluded_features),
        }
    }
}
