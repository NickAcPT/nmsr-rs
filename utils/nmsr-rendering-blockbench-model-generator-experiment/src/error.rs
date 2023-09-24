use image::ImageError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockbenchGeneratorError {
    #[error("{1}: {0}")]
    ImageError(image::error::ImageError, &'static str),
    #[cfg(feature = "ears")]
    #[error("Unable to parse ears features: {0}")]
    EarsError(#[from] ears_rs::utils::errors::EarsError),
    #[error("Failed to find texture id for {0:?}")]
    TextureNotFound(nmsr_rendering::high_level::types::PlayerPartTextureType),
    #[cfg(feature = "wasm")]
    #[error("Failed to serialize project: {0}")]
    SerdeWasmBindgenError(#[from] serde_wasm_bindgen::Error),
    #[cfg(not(feature = "wasm"))]
    #[error("Failed to serialize project: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("{0}")]
    ExplainedError(String),
    
}

pub trait Contextualizable<O> {
    fn context(self, context: &'static str) -> O;
}

impl<T> Contextualizable<Result<T>> for std::result::Result<T, ImageError> {
    fn context(self, context: &'static str) -> Result<T> {
        self.map_err(|e| BlockbenchGeneratorError::ImageError(e, context))
    }
}

pub(crate) type Result<T> = std::result::Result<T, BlockbenchGeneratorError>;
