use crate::utils::errors::NMSRaaSError;
use crate::utils::Result;

#[derive(Debug, Clone)]
pub(crate) enum PlayerRenderRequest {
    PlayerUuid(uuid::Uuid),
    TextureHash(String)
}

impl TryFrom<String> for PlayerRenderRequest {
    type Error = NMSRaaSError;

    fn try_from(value: String) -> Result<PlayerRenderRequest> {
        if value.len() == 32 || value.len() == 36 {
            let uuid = uuid::Uuid::parse_str(&value).map_err(NMSRaaSError::InvalidUUID)?;
            Ok(PlayerRenderRequest::PlayerUuid(uuid))
        } else if value.len() > 36 {
            Ok(PlayerRenderRequest::TextureHash(value))
        } else {
            Err(NMSRaaSError::InvalidPlayerRequest(value))
        }
    }
}