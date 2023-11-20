use super::mojang::client::MojangClient;
use crate::{
    error::{MojangRequestError, MojangRequestResult},
    model::request::entry::RenderRequestEntryModel,
};
use hyper::Method;
use serde::Deserialize;
use tracing::{instrument, Span};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct GeyserSkinResponse {
    is_steve: bool,
    texture_id: String,
}

#[instrument(skip(client))]
pub async fn resolve_geyser_uuid_to_texture_and_model(
    client: &MojangClient,
    uuid: &Uuid,
) -> MojangRequestResult<(String, RenderRequestEntryModel)> {
    let xuid = u64::from_str_radix(&uuid.simple().to_string(), 16)
        .map_err(|_| MojangRequestError::UnableToParseUuidIntoXuid(*uuid))?;

    let url = format!(
        "{geysermc_api_server}/v2/skin/{xuid}",
        geysermc_api_server = client.mojank_config().geysermc_api_server
    );

    let bytes = client
        .do_request(&url, Method::GET, &Span::current(), || None)
        .await?;

    let response: GeyserSkinResponse = serde_json::from_slice(&bytes)?;

    let model = if response.is_steve {
        RenderRequestEntryModel::Steve
    } else {
        RenderRequestEntryModel::Alex
    };

    Ok((response.texture_id, model))
}
