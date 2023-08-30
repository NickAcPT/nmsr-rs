use axum::response::IntoResponse;
use hyper::StatusCode;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum NMSRaaSError {
    #[error("Invalid player request: {0}")]
    RenderRequestError(#[from] RenderRequestError),
    #[error("Model cache error: {0}")]
    ModelCacheError(#[from] ModelCacheError),
    #[error("Mojang request error: {0}")]
    MojangRequestError(#[from] MojangRequestError),
    #[error("Render error: {0}")]
    RenderError(#[from] nmsr_rendering::errors::NMSRRenderingError),
}

#[derive(Error, Debug)]
pub enum RenderRequestError {
    #[error("Invalid UUID: {0}")]
    InvalidUUID(#[from] uuid::Error),
    #[error("The UUID you requested ({0}) has version {1} instead of version 4. Version 4 UUIDs are required for online player skins.")]
    InvalidPlayerUuidRequest(String, usize),
    #[error("{0}")]
    InvalidPlayerRequest(String),
    #[error("Io error: {0}")]
    ExplainedIoError(std::io::Error, String),
    #[error("Path Rejection Error: {0}")]
    PathRejection(#[from] axum::extract::rejection::PathRejection),
    #[error("Query Rejection Error: {0}")]
    QueryRejection(#[from] axum::extract::rejection::QueryRejection),
    #[error("Invalid render mode: {0}")]
    InvalidRenderMode(String),
    #[error("Unable to upgrade legacy skin to modern format")]
    LegacySkinUpgradeError,
    #[error("The render setting you've specified ({0}) is invalid. Valid values should be {1}.")]
    InvalidRenderSettingError(&'static str, String),
    #[error("You've specified {0} which is invalid for this mode. {1}")]
    InvalidModeSettingSpecifiedError(&'static str, &'static str),
}

#[derive(Error, Debug)]
pub enum ModelCacheError {
    #[error("Unable to read marker for entry {0:?}")]
    MarkerMetadataError(crate::model::request::entry::RenderRequestEntry),
    #[error("Invalid player request attempt: {0}")]
    InvalidRequestCacheAttempt(String),
    #[error("Invalid cache entry marker request: {0}")]
    InvalidCacheEntryMarkerRequest(String),
    #[error("Invalid cache bias configuration: {0}")]
    InvalidCacheBiasConfiguration(String),
}

#[derive(Error, Debug)]
pub enum MojangRequestError {
    #[error("Unable to decode game profile from base64: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("Unable to decode game profile from utf8: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Unable to decode game profile from json: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Game profile is missing the textures property")]
    MissingTexturesPropertyError,
    #[error("Game profile has an invalid textures property: {0}")]
    InvalidTexturesPropertyError(serde_json::Error),
    #[error("Url parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Http error: {0}")]
    HttpRequestError(#[from] hyper::http::Error),
    #[error("Request error: {0}")]
    RequestError(#[from] hyper::Error),
    #[error("Missing skin from game profile: {0}")]
    MissingSkinPropertyError(Uuid),
    #[error("Received invalid texture url: {0}")]
    InvalidTextureUrlError(String),
    #[error("Received result while fetching from mojang: {0}")]
    MojangFetchRequestError(String),
    #[error("Unable to resolve render request entity {1:?}: {0}")]
    UnableToResolveRenderRequestEntity(Box<dyn std::error::Error + Send + Sync>, crate::model::request::entry::RenderRequestEntry),
}

pub(crate) type Result<T> = std::result::Result<T, NMSRaaSError>;
pub(crate) type RenderRequestResult<T> = std::result::Result<T, RenderRequestError>;
pub(crate) type ModelCacheResult<T> = std::result::Result<T, ModelCacheError>;
pub(crate) type MojangRequestResult<T> = std::result::Result<T, MojangRequestError>;

pub trait ExplainableExt<T> {
    fn explain_closure<O: FnOnce() -> String>(self, message: O) -> Result<T>;
    
    fn explain(self, message: String) -> Result<T> where Self: Sized {
        self.explain_closure(move || message)
    }
}

impl<T> ExplainableExt<T> for std::result::Result<T, std::io::Error> {
    fn explain_closure<O: FnOnce() -> String>(self, message: O) -> Result<T> {
        self.map_err(|e| RenderRequestError::ExplainedIoError(e, message()).into())
    }
}

pub struct NmsrErrorExtension(pub NMSRaaSError);

impl IntoResponse for NMSRaaSError {
    fn into_response(self) -> axum::response::Response {
        let mut res = axum::response::IntoResponse::into_response(self.to_string());
        *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        
        res.extensions_mut().insert(NmsrErrorExtension(self));
        
        res
    }
}