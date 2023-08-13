use actix_web::http::StatusCode;
#[cfg(feature = "tracing")]
use opentelemetry::trace::TraceError;
use std::string::FromUtf8Error;
use std::sync::PoisonError;

use thiserror::Error;

#[cfg(feature = "uv")]
use nmsr_lib::vfs::VfsError;

#[cfg(feature = "uv")]
use crate::manager::RenderMode;

#[derive(Error, Debug)]
pub(crate) enum NMSRaaSError {
    #[error("Invalid UUID: {0}")]
    InvalidUUID(#[from] uuid::Error),
    #[error("Invalid player request: {0}")]
    InvalidPlayerRequest(String),
    #[error("Invalid player request: The UUID you requested ({0}) has version {1} instead of version 4. Version 4 UUIDs are required for online player skins.")]
    InvalidPlayerUuidRequest(String, usize),
    #[error("Invalid player game profile response: {0}")]
    MojangRequestError(#[from] reqwest::Error),
    #[error("Invalid player game profile response: {0}")]
    GameProfileError(String),
    #[error("Missing textures property from player game profile")]
    MissingTexturesProperty,
    #[error("Invalid base64 texture data")]
    InvalidBase64TexturesProperty,
    #[error("Failed to decode textures property from player game profile (base64): {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
    #[error("Failed to decode textures property from player game profile (json): {0}")]
    InvalidJsonError(#[from] serde_json::Error),
    #[error("Invalid skin hash url: {0}")]
    InvalidHashTextureUrl(String),
    #[error("Invalid skin: {0}")]
    InvalidImageError(#[from] image::ImageError),
    #[error("NMSR error: {0}")]
    #[cfg(feature = "uv")]
    NMSRError(#[from] nmsr_lib::errors::NMSRError),
    #[error("IO error: {1} -> {0}")]
    ExplainedIOError(std::io::Error, String),
    #[error("IO error: {0}")]
    #[cfg(feature = "uv")]
    VirtualIOError(VfsError),
    #[error("System time error: {0}")]
    SystemTimeError(#[from] std::time::SystemTimeError),
    #[error("Failed to accquire lock on cache manager")]
    MutexPoisonError,
    #[error("Failed to find part manager for mode: {0}")]
    #[cfg(feature = "uv")] 
    MissingPartManager(RenderMode),
    #[error("Invalid render mode: {0}")]
    InvalidRenderMode(String),
    #[error("Failed to walk directory: {0}")]
    WalkDirError(#[from] walkdir::Error),
    #[error("TLS error: {0}")]
    TlsError(#[from] rustls::Error),
    #[error("Error decoding toml: {0}")]
    TomlDecodeError(#[from] toml::de::Error),
    #[error("Bincode Error: {0}")]
    BincodeError(#[from] Box<bincode::ErrorKind>),
    #[cfg(feature = "tracing")]
    #[error("Trace error: {0}")]
    TraceError(#[from] TraceError),
    #[error("Reqwest middleware error: {0}")]
    ReqwestMiddlewareError(#[from] reqwest_middleware::Error),
    #[error("Log Tracing error: {0}")]
    LogTracingError(#[from] tracing_log::log::SetLoggerError),
    #[error("Legacy Skin upgrade error")]
    LegacySkinUpgradeError,
    #[error("Nmsr Rendering error: {0}")]
    NMSRRenderingError(#[from] nmsr_rendering::errors::NMSRRenderingError),
}

impl actix_web::error::ResponseError for NMSRaaSError {
    fn status_code(&self) -> StatusCode {
        match self {
            NMSRaaSError::InvalidPlayerUuidRequest(_, _) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

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

#[cfg(feature = "uv")]
impl From<VfsError> for NMSRaaSError {
    fn from(e: VfsError) -> Self {
        NMSRaaSError::VirtualIOError(e)
    }
}

pub trait ExplainableIoError<R> {
    fn explain<S>(self, explanation: S) -> R where S: Into<String>;
}

impl ExplainableIoError<NMSRaaSError> for std::io::Error {
    fn explain<S>(self, explanation: S) -> NMSRaaSError where S: Into<String> {
        NMSRaaSError::ExplainedIOError(self, explanation.into())
    }
}

impl<T> ExplainableIoError<std::result::Result<T, NMSRaaSError>> for std::result::Result<T, std::io::Error> {
    fn explain<S>(self, explanation: S) -> std::result::Result<T, NMSRaaSError> where S: Into<String> {
        self.map_err(|e| e.explain(explanation))
    }
}