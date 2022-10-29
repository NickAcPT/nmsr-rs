use thiserror::Error;
use uuid::Error;

#[derive(Error, Debug)]
pub(crate) enum NMSRaaSError {
    #[error("Invalid UUID: {0}")]
    InvalidUUID(#[from] Error),
    #[error("Invalid player request: {0}")]
    InvalidPlayerRequest(String),
}

impl actix_web::error::ResponseError for NMSRaaSError {}