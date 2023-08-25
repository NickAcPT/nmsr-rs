use axum::extract::{Path, State};
use hyper::body::Bytes;

use crate::error::Result;

use super::NMSRState;

pub async fn get_skin(
    Path(texture): Path<String>,
    State(state): State<NMSRState>,
) -> Result<Bytes> {
    Ok(Bytes::from(vec![]))
}
