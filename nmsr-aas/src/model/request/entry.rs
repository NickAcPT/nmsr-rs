use derive_more::Debug;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Default, Clone, Copy, strum::FromRepr, strum::EnumCount)]
pub enum RenderRequestEntryModel {
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
