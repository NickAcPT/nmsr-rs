use axum::extract::State;

use crate::{error::Result, model::request::RenderRequest};

use super::NMSRState;

pub async fn get_skin(request: RenderRequest, State(_state): State<NMSRState>) -> Result<String> {
    Ok(format!("{:#?}", request))
}
