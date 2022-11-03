use std::path::PathBuf;
use thiserror::Error;
use crate::uv::part::Point;

#[derive(Error, Debug)]
pub enum NMSRError {
    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),
    #[error("IO error ({1}): {0}")]
    IoError(std::io::Error, String),
    #[error("IO error: {0}")]
    UnspecifiedIoError(String),
    #[error("Image error: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("Expected there to be at least 1 UV part")]
    NoPartsFound,
    #[error("An error occurred while upgrading legacy skin to modern format")]
    LegacySkinUpgradeError,
    #[error("Invalid UV Point: {0}")]
    InvalidUvPoint(Point<u8>),
    #[error("Unspecified NMSR error: {0}")]
    UnspecifiedNMSRError(String),
}

pub type Result<T> = std::result::Result<T, NMSRError>;

impl From<&NMSRError> for NMSRError {
    fn from(e: &NMSRError) -> Self {
        NMSRError::UnspecifiedNMSRError(e.to_string())
    }
}
