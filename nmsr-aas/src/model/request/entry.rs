use derive_more::Debug;
use indoc::formatdoc;
use nmsr_rendering::high_level::model::PlayerModel;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use strum::{Display, EnumCount, EnumString, FromRepr};
use uuid::Uuid;

use crate::error::{RenderRequestError, RenderRequestResult};

#[derive(Clone, PartialEq, Eq, Hash, Debug, Deserialize, Serialize)]
pub enum RenderRequestEntry {
    MojangPlayerUuid(Uuid),
    MojangOfflinePlayerUuid(Uuid),
    GeyserPlayerUuid(Uuid),
    TextureHash(String),
    PlayerSkin(#[debug(skip)] Vec<u8>, #[debug(skip)] Option<Vec<u8>>),
}

impl RenderRequestEntry {
    pub fn new_from_skin_and_cape(skin: Vec<u8>, cape: Option<Vec<u8>>) -> Self {
        Self::PlayerSkin(skin, cape)
    }
}

static VALID_TEXTURE_HASH_REGEX: OnceLock<regex::Regex> = OnceLock::new();

impl TryFrom<String> for RenderRequestEntry {
    type Error = RenderRequestError;

    fn try_from(value: String) -> RenderRequestResult<Self> {
        if value.len() == 32 || value.len() == 36 {
            let uuid = Uuid::parse_str(&value).map_err(RenderRequestError::InvalidUUID)?;
            let uuid_version = uuid.get_version_num();

            if uuid_version == 4 {
                Ok(Self::MojangPlayerUuid(uuid))
            } else if uuid_version == 3 {
                Ok(Self::MojangOfflinePlayerUuid(uuid))
            } else if uuid_version == 0 {
                Ok(Self::GeyserPlayerUuid(uuid))
            } else {
                Err(RenderRequestError::InvalidPlayerUuidRequest(
                    value,
                    uuid_version,
                ))
            }
        } else if value.len() > 36 {
            let regex = VALID_TEXTURE_HASH_REGEX
                .get_or_init(|| regex::Regex::new(r"^[a-f0-9]{36,64}$").unwrap());

            if !regex.is_match(&value) {
                return Err(RenderRequestError::InvalidPlayerRequest(formatdoc! {"
                    You've provided an invalid texture hash ({value}).
                    Texture hashes should be 36-64 characters long and only contain the characters 0-9 and a-f.
                    
                    Perhaps you meant to use a question mark (`?`) instead of an ampterstand (`&`) for the first query parameter separator? 
                    Doing so will cause the server to interpret the texture argument as a texture hash even if it's a valid UUID.
                    
                    If you're using a texture hash, make sure that what you provided is a valid texture hash.
                    You can check this by using the following regular expression: ^[a-f0-9]{{36,64}}$
                "}));
            }

            Ok(Self::TextureHash(value))
        } else {
            Err(RenderRequestError::InvalidPlayerRequest(formatdoc! {"
                You've provided an invalid player request ({value}).
                I don't know what to do with this.
                
                If it's a texture hash, make sure that it's a valid texture hash.
                If you've provided a UUID, make sure that it's a valid UUID and isn't truncated.
                Otherwise, if you're using a player name, you should resolve it to a UUID first.
            "}))
        }
    }
}

impl TryFrom<RenderRequestEntry> for String {
    type Error = RenderRequestError;

    fn try_from(value: RenderRequestEntry) -> Result<Self, Self::Error> {
        match value {
            RenderRequestEntry::MojangPlayerUuid(uuid)
            | RenderRequestEntry::MojangOfflinePlayerUuid(uuid)
            | RenderRequestEntry::GeyserPlayerUuid(uuid) => Ok(uuid.to_string()),
            RenderRequestEntry::TextureHash(hash) => Ok(hash),
            RenderRequestEntry::PlayerSkin(_, _) => Err(RenderRequestError::InvalidPlayerRequest(
                "Unable to convert PlayerSkin to String".to_string(),
            )),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, FromRepr, Display, EnumString, EnumCount, PartialEq, Eq)]
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
            RenderRequestEntryModel::Steve => Self::Steve,
            RenderRequestEntryModel::Alex => Self::Alex,
        }
    }
}
