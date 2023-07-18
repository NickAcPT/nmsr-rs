use actix_web::web::Bytes;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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
    pub(crate) skin: Skin,
}

#[derive(Debug, Serialize, Deserialize)]
struct Skin {
    pub(crate) url: String,
    pub(crate) metadata: Option<SkinMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SkinMetadata {
    pub(crate) model: String,
}

#[derive(Debug, Clone)]
pub(crate) enum CachedSkinHash {
    WithoutModel { skin_hash: String },
    WithModel { skin_hash: String, slim_arms: bool },
}

impl CachedSkinHash {
    pub(crate) fn get_hash(&self) -> &String {
        match self {
            CachedSkinHash::WithoutModel { skin_hash } => skin_hash,
            CachedSkinHash::WithModel { skin_hash, .. } => skin_hash,
        }
    }
    
    pub(crate) fn is_slim_arms(&self) -> bool {
        match self {
            CachedSkinHash::WithoutModel { .. } => false,
            CachedSkinHash::WithModel { slim_arms, .. } => *slim_arms
        }
    }
}

impl GameProfile {
    fn get_textures(&self) -> Result<GameProfileTextures> {
        let textures = self
            .properties
            .iter()
            .find(|property| property.name == "textures")
            .ok_or(NMSRaaSError::MissingTexturesProperty)?;

        let decoded = base64::decode(&textures.value)?;
        let decoded = String::from_utf8(decoded)?;

        Ok(serde_json::from_str(&decoded)?)
    }
}

#[cfg_attr(feature = "tracing", instrument(level="trace", skip(client)))]
async fn get_player_game_profile(client: &Client, id: Uuid) -> Result<GameProfile> {
    let response = client
        .get(format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{}",
            id
        ))
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

#[cfg_attr(feature = "tracing", instrument(level="trace", skip(client, rate_limiter)))]
pub(crate) async fn get_skin_hash_and_model(
    client: &Client,
    rate_limiter: &RateLimiterType,
    id: Uuid,
) -> Result<CachedSkinHash> {
    rate_limiter.until_ready().await;

    let game_profile = get_player_game_profile(client, id).await?;
    let textures = game_profile.get_textures()?;
    let skin = textures.textures.skin;

    let url = skin.url;

    let slim = skin.metadata.map(|m| m.model == "slim").unwrap_or(false);

    // Take only after last slash
    let hash = get_skin_hash_from_url(url)?;

    Ok(CachedSkinHash::WithModel {
        skin_hash: hash,
        slim_arms: slim,
    })
}

pub(crate) fn get_skin_hash_from_url(url: String) -> Result<String> {
    Ok(url
        .split('/')
        .last()
        .ok_or_else(|| NMSRaaSError::InvalidHashSkinUrl(url.to_string()))?
        .to_string())
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip(client)))]
pub(crate) async fn fetch_skin_bytes_from_mojang(hash: &String, client: &Client) -> Result<Bytes> {
    let response = client
        .get(format!("http://textures.minecraft.net/texture/{}", hash))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(NMSRaaSError::InvalidHashSkinUrl(hash.to_string()));
    }

    let bytes = response.bytes().await?;

    Ok(bytes)
}
