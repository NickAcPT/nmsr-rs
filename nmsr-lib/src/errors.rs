use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NMSRError {
    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Image error: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("Expected there to be at least 1 UV part")]
    NoPartsFound,
}

pub type Result<T> = std::result::Result<T, NMSRError>;
