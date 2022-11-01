use std::io::Error;
use std::string::FromUtf8Error;
use std::sync::PoisonError;
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
    #[error("Invalid base64 texture data")]
    InvalidBase64TexturesProperty,
    #[error("Failed to decode textures property from player game profile (base64): {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
    #[error("Failed to decode textures property from player game profile (json): {0}")]
    InvalidJsonError(#[from] serde_json::Error),
    #[error("Invalid skin hash url: {0}")]
    InvalidHashSkinUrl(String),
    #[error("Invalid skin: {0}")]
    InvalidImageError(#[from] image::ImageError),
    #[error("NMSR error: {0}")]
    NMSRError(#[from] nmsr_lib::errors::NMSRError),
    #[error("IO error: {0}")]
    IOError(#[from] Error),
    #[error("System time error: {0}")]
    SystemTimeError(#[from] std::time::SystemTimeError),
    #[error("Failed to accquire lock on cache manager")]
    MutexPoisonError,
}

impl actix_web::error::ResponseError for NMSRaaSError {}

impl From<FromUtf8Error> for NMSRaaSError {
    fn from(_: FromUtf8Error) -> Self {
        NMSRaaSError::InvalidBase64TexturesProperty
    }
}

impl<T> From<PoisonError<T>> for NMSRaaSError {
    fn from(_: PoisonError<T>) -> Self {
        NMSRaaSError::MutexPoisonError
    }
}
