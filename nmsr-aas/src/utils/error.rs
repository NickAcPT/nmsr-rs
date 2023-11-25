use std::path::PathBuf;

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
    #[error("Armor manager error: {0}")]
    ArmorManagerError(#[from] ArmorManagerError),
    
    #[error("{0}")]
    ClonedError(String),

    #[cfg(feature = "ears")]
    #[error("Ears error: {0}")]
    EarsError(#[from] ears_rs::utils::errors::EarsError),

    #[error("Unable to generate blockbench project: {0}")]
    BlockbenchGeneratorError(
        #[from]
        nmsr_rendering_blockbench_model_generator_experiment::error::BlockbenchGeneratorError,
    ),
}

#[derive(Error, Debug)]
pub enum RenderRequestError {
    #[error("Invalid UUID: {0}")]
    InvalidUUID(#[from] uuid::Error),
    #[error("The UUID you requested ({0}) has version {1} instead of version 4. Version 4 UUIDs are required for online player skins.")]
    InvalidPlayerUuidRequest(String, usize),
    #[error("{0}")]
    InvalidPlayerRequest(String),
    #[error("Io error: {0} ({1})")]
    ExplainedIoError(std::io::Error, String),
    #[error("Path Rejection Error: {0}")]
    PathRejection(#[from] axum::extract::rejection::PathRejection),
    #[error("Query Rejection Error: {0}")]
    QueryRejection(#[from] axum::extract::rejection::QueryRejection),
    #[error("Multipart Error: {0}")]
    MultipartError(#[from] axum_extra::extract::multipart::MultipartError),
    #[error("Multipart Rejection: {0}")]
    MultipartRejection(#[from] axum_extra::extract::multipart::MultipartRejection),
    #[error("Unable to decode multipart: {0} ({1})")]
    MultipartDecodeError(serde_json::Error, serde_json::Value),
    #[error("Invalid render mode: {0}")]
    InvalidRenderMode(String),
    #[error("Unable to upgrade legacy skin to modern format")]
    LegacySkinUpgradeError,
    #[error("The render setting you've specified ({0}) is invalid. Valid values should be {1}.")]
    InvalidRenderSettingError(&'static str, String),
    #[error("You've specified {0} which is invalid for this mode. {1}")]
    InvalidModeSettingSpecifiedError(&'static str, &'static str),
    #[error("Missing render request texture. Did you forget to specify a texture?")]
    MissingRenderRequestEntry,
    #[error("Invalid HTTP Method. Did you mean to use \"{1}\" instead of \"{0}\"? This endpoint only supports \"{0}\".")]
    WrongHttpMethodError(&'static str, &'static str),
}

impl RenderRequestError {
    #[must_use]
    pub const fn is_bad_request(&self) -> bool {
        matches!(
            self,
            Self::InvalidUUID(_)
                | Self::InvalidPlayerUuidRequest(_, _)
                | Self::InvalidPlayerRequest(_)
                | Self::InvalidRenderMode(_)
                | Self::InvalidRenderSettingError(_, _)
                | Self::InvalidModeSettingSpecifiedError(_, _)
                | Self::MissingRenderRequestEntry
                | Self::WrongHttpMethodError(_, _)
        )
    }
}

#[derive(Error, Debug)]
pub enum ModelCacheError {
    #[error("Unable to read marker for entry {0:?}")]
    MarkerMetadataError(crate::model::request::entry::RenderRequestEntry),
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
    BoxedRequestError(Box<dyn std::error::Error + Send + Sync>),
    #[error("Request error: {0}")]
    RequestError(#[from] hyper_util::client::legacy::Error),
    #[error("Missing skin from game profile: {0}")]
    MissingSkinPropertyError(Uuid),
    #[error("Received invalid texture url: {0}")]
    InvalidTextureUrlError(String),
    #[error("Received result while fetching from mojang: {0}")]
    MojangFetchRequestError(String),
    #[error("Unable to resolve render request entity {1:?}: {0}")]
    UnableToResolveRenderRequestEntity(
        Box<dyn std::error::Error + Send + Sync>,
        crate::model::request::entry::RenderRequestEntry,
    ),
    #[error("Unable to parse uuid {0} into xuid")]
    UnableToParseUuidIntoXuid(Uuid),
    #[error("The texture hash you provided ({0}) is invalid. Are you sure this texture hash comes from Mojang? You can't hash a skin and expect it to work.")]
    InvalidTextureHashError(String),
    #[error("Unable to find a player with the UUID {0}")]
    GameProfileNotFound(Uuid),
}

#[derive(Error, Debug)]
pub enum ArmorManagerError {
    #[error("Unable to parse armor: {0}")]
    ArmorParseError(#[from] strum::ParseError),
    #[error("Missing Armor texture: {0:?}")]
    MissingArmorTextureError(PathBuf),
    #[error("Unable to load armor texture for {0:?}: {1}")]
    ArmorTextureLoadError(PathBuf, image::error::ImageError),
    #[error("Unable to upgrade armor texture to 64x64")]
    ArmorTextureUpgradeError,
    #[error("Empty armor slot")]
    EmptyArmorSlotError,
    #[error("Unknown partial armor material name: {0}")]
    UnknownPartialArmorMaterialName(String),
    #[error("Invalid trim count: {0}")]
    InvalidTrimCountError(usize),
}

pub(crate) type Result<T> = std::result::Result<T, NMSRaaSError>;
pub(crate) type RenderRequestResult<T> = std::result::Result<T, RenderRequestError>;
pub(crate) type ModelCacheResult<T> = std::result::Result<T, ModelCacheError>;
pub(crate) type MojangRequestResult<T> = std::result::Result<T, MojangRequestError>;
pub(crate) type ArmorManagerResult<T> = std::result::Result<T, ArmorManagerError>;

pub trait ExplainableExt<T> {
    fn explain_closure<O: FnOnce() -> String>(self, message: O) -> Result<T>;

    fn explain(self, message: String) -> Result<T>
    where
        Self: Sized,
    {
        self.explain_closure(move || message)
    }
}

impl<T> ExplainableExt<T> for std::result::Result<T, std::io::Error> {
    fn explain_closure<O: FnOnce() -> String>(self, message: O) -> Result<T> {
        self.map_err(|e| RenderRequestError::ExplainedIoError(e, message()).into())
    }
}

pub struct NmsrErrorExtension(pub NMSRaaSError);

impl Clone for NmsrErrorExtension {
    fn clone(&self) -> Self {
        // This is bad code, I don't care
        Self(NMSRaaSError::ClonedError(self.0.to_string()))
    }
}

impl IntoResponse for NMSRaaSError {
    fn into_response(self) -> axum::response::Response {
        let mut res = axum::response::IntoResponse::into_response(self.to_string());

        let is_bad_request = if let Self::RenderRequestError(error) = &self {
            error.is_bad_request()
        } else {
            false
        };

        let error = if is_bad_request {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        *res.status_mut() = error;

        res.extensions_mut().insert(NmsrErrorExtension(self));

        res
    }
}
