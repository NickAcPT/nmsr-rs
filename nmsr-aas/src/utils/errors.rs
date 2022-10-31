use std::io::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum NMSRaaSError {
    #[error("Invalid UUID: {0}")]
    InvalidUUID(#[from] uuid::Error),
    #[error("Invalid player request: {0}")]
    InvalidPlayerRequest(String),
    #[error("Invalid player game profile response: {0}")]
    MojangRequestError(#[from] reqwest::Error),
    #[error("Missing textures property from player game profile")]
    MissingTexturesProperty,
    #[error("Failed to decode textures property from player game profile")]
    Base64DecodeError(#[from] base64::DecodeError),
    #[error("Failed to decode textures property from player game profile")]
    InvalidJsonError(#[from] serde_json::Error),
    #[error("Invalid hash skin url")]
    InvalidHashSkinUrl,
    #[error("Invalid skin: {0}")]
    InvalidImageError(#[from] image::ImageError),
    #[error("NMSR error: {0}")]
    NMSRError(#[from] nmsr_lib::errors::NMSRError),
    #[error("IO error: {0}")]
    IOError(#[from] Error),
}

impl actix_web::error::ResponseError for NMSRaaSError {}