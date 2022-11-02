use std::path::PathBuf;
use thiserror::Error;

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
}

pub type Result<T> = std::result::Result<T, NMSRError>;
