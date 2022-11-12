use actix_web::web::Bytes;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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

pub(crate) async fn get_skin_hash(
    client: &Client,
    rate_limiter: &RateLimiterType,
    id: Uuid,
) -> Result<String> {
    rate_limiter.until_ready().await;

    let game_profile = get_player_game_profile(client, id).await?;
    let textures = game_profile.get_textures()?;
    let url = textures.textures.skin.url;

    // Take only after last slash
    let hash = get_skin_hash_from_url(url)?;

    Ok(hash)
}

pub(crate) fn get_skin_hash_from_url(url: String) -> Result<String> {
    Ok(url
        .split('/')
        .last()
        .ok_or_else(|| NMSRaaSError::InvalidHashSkinUrl(url.to_string()))?
        .to_string())
}

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
