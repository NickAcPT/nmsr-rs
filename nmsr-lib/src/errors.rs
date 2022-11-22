use std::path::PathBuf;

use thiserror::Error;
use vfs::VfsError;

use crate::geometry::Point;

#[derive(Error, Debug)]
pub enum NMSRError {
    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),
    #[error("IO error ({1}): {0}")]
    IoError(VfsError, String),
    #[error("Virtual IO error: {0}")]
    VirtualIoError(VfsError),
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
    #[error("Ears parse error: {0}")]
    EarsError(#[from] ears_rs::utils::errors::EarsError),
}

pub type Result<T> = std::result::Result<T, NMSRError>;

impl From<&NMSRError> for NMSRError {
    fn from(e: &NMSRError) -> Self {
        NMSRError::UnspecifiedNMSRError(e.to_string())
    }
}

impl From<VfsError> for NMSRError {
    fn from(e: VfsError) -> Self {
        NMSRError::VirtualIoError(e)
    }
}
