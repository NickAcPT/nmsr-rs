use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
    RequestPartsExt,
};
use enumset::EnumSet;

use crate::{
    error::{NMSRaaSError, RenderRequestError, Result},
    model::request::{entry::RenderRequestEntry, RenderRequest},
};

#[async_trait]
impl<S> FromRequestParts<S> for RenderRequest
where
    S: Send + Sync,
{
    type Rejection = NMSRaaSError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        let Path(entry_str): Path<String> = parts
            .extract_with_state::<Path<String>, S>(state)
            .await
            .map_err(RenderRequestError::from)?;

        let entry = RenderRequestEntry::try_from(entry_str)?;

        Ok(RenderRequest::new_from_excluded_features(
            entry,
            None,
            EnumSet::EMPTY,
        ))
    }
}
