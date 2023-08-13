use actix_web::web::Bytes;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
#[cfg(feature = "tracing")]
use tracing::instrument;
use uuid::Uuid;

use crate::mojang::caching::RateLimiterType;
use crate::utils::errors::NMSRaaSError;
use crate::utils::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GameProfileProperty {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GameProfile {
    pub id: String,
    pub name: String,
    pub properties: Vec<GameProfileProperty>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GameProfileTextures {
    pub(crate) textures: Textures,
}

#[derive(Debug, Serialize, Deserialize)]
struct Textures {
    #[serde(rename = "SKIN")]
    pub(crate) skin: Texture,
    #[serde(rename = "CAPE")]
    pub(crate) cape: Option<Texture>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Texture {
    pub(crate) url: String,
    pub(crate) metadata: Option<SkinMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SkinMetadata {
    pub(crate) model: String,
}

#[derive(Debug, Clone)]
pub(crate) struct UnwrappedGameProfileMetadata {
    pub(crate) skin_texture_hash: String,
    pub(crate) cape_texture_hash: Option<String>,
    pub(crate) slim_arms: bool,
}
impl GameProfile {
    fn get_textures(&self) -> Result<GameProfileTextures> {
        let textures = self
            .properties
            .iter()
            .find(|property| property.name == "textures")
            .ok_or(NMSRaaSError::MissingTexturesProperty)?;

        let decoded = STANDARD.decode(&textures.value)?;
        let decoded = String::from_utf8(decoded)?;

        Ok(serde_json::from_str(&decoded)?)
    }
}

#[cfg_attr(
    feature = "tracing",
    instrument(level = "trace", skip(client, session_server))
)]
async fn get_player_game_profile(
    client: &ClientWithMiddleware,
    session_server: &String,
    id: Uuid,
) -> Result<GameProfile> {
    let response = client
        .get(format!("{session_server}/session/minecraft/profile/{id}"))
        .send()
        .await?;

    if !response.status().is_success() {
        Err(NMSRaaSError::GameProfileError(format!(
            "Failed to fetch game profile for {}: {}",
            id,
            response.status()
        )))
    } else {
        Ok(response.json::<GameProfile>().await?)
    }
}

#[cfg_attr(
    feature = "tracing",
    instrument(level = "trace", skip(client, rate_limiter, id, session_server))
)]
pub(crate) async fn get_unwrapped_gameprofile(
    client: &ClientWithMiddleware,
    rate_limiter: &RateLimiterType,
    id: Uuid,
    session_server: &String,
) -> Result<UnwrappedGameProfileMetadata> {
    rate_limiter.until_ready().await;

    let game_profile = get_player_game_profile(client, session_server, id).await?;
    let gameprofile_textures = game_profile.get_textures()?;
    let textures = gameprofile_textures.textures;

    let slim_arms = textures.skin.metadata.map(|m| m.model == "slim").unwrap_or(false);

    let skin_texture_hash = get_texture_hash_from_url(textures.skin.url)?;
    
    let cape_texture_hash = textures.cape.and_then(|t| get_texture_hash_from_url(t.url).ok());

    Ok(UnwrappedGameProfileMetadata { skin_texture_hash , cape_texture_hash, slim_arms })
}

pub(crate) fn get_texture_hash_from_url(url: String) -> Result<String> {
    Ok(url
        .split('/')
        .last()
        .ok_or_else(|| NMSRaaSError::InvalidHashTextureUrl(url.to_string()))?
        .to_string())
}

#[cfg_attr(
    feature = "tracing",
    tracing::instrument(skip(hash, client, textures_server))
)]
pub(crate) async fn fetch_texture_from_mojang(
    hash: &str,
    client: &ClientWithMiddleware,
    textures_server: &String,
) -> Result<Bytes> {
    let response = client
        .get(format!("{textures_server}/texture/{hash}"))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(NMSRaaSError::InvalidHashTextureUrl(hash.to_string()));
    }

    let bytes = response.bytes().await?;

    Ok(bytes)
}
