use derive_more::Debug;
use nmsr_rendering::high_level::player_model::PlayerModel;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumCount, EnumString, FromRepr};
use uuid::Uuid;

use crate::error::{RenderRequestError, RenderRequestResult};

#[derive(Clone, PartialEq, Eq, Hash, Debug, Deserialize, Serialize)]
pub enum RenderRequestEntry {
    PlayerUuid(Uuid),
    TextureHash(String),
    PlayerSkin(#[debug(skip)] Vec<u8>),
}

impl TryFrom<String> for RenderRequestEntry {
    type Error = RenderRequestError;

    fn try_from(value: String) -> RenderRequestResult<RenderRequestEntry> {
        if value.len() == 32 || value.len() == 36 {
            let uuid = Uuid::parse_str(&value).map_err(RenderRequestError::InvalidUUID)?;
            let uuid_version = uuid.get_version_num();

            if uuid_version == 4 {
                Ok(RenderRequestEntry::PlayerUuid(uuid))
            } else {
                Err(RenderRequestError::InvalidPlayerUuidRequest(
                    value,
                    uuid_version,
                ))
            }
        } else if value.len() > 36 {
            Ok(RenderRequestEntry::TextureHash(value))
        } else {
            Err(RenderRequestError::InvalidPlayerRequest(value))
        }
    }
}

impl TryFrom<RenderRequestEntry> for String {
    type Error = RenderRequestError;

    fn try_from(value: RenderRequestEntry) -> Result<Self, Self::Error> {
        match value {
            RenderRequestEntry::PlayerUuid(uuid) => Ok(uuid.to_string()),
            RenderRequestEntry::TextureHash(hash) => Ok(hash),
            RenderRequestEntry::PlayerSkin(_) => Err(RenderRequestError::InvalidPlayerRequest(
                "Unable to convert PlayerSkin to String".to_string(),
            )),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, FromRepr, Display, EnumString, EnumCount, PartialEq)]
pub enum RenderRequestEntryModel {
    #[default]
    #[strum(serialize = "steve", serialize = "wide")]
    Steve,
    #[strum(serialize = "alex", serialize = "slim")]
    Alex,
}

impl From<RenderRequestEntryModel> for PlayerModel {
    fn from(value: RenderRequestEntryModel) -> Self {
        match value {
            RenderRequestEntryModel::Steve => PlayerModel::Steve,
            RenderRequestEntryModel::Alex => PlayerModel::Alex,
        }
    }
}
