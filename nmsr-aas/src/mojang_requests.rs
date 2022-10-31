use actix_web::web::Bytes;
use cached::proc_macro::cached;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::utils::errors::NMSRaaSError;
use crate::utils::Result;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct GameProfileProperty {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct GameProfile {
    pub id: String,
    pub name: String,
    pub properties: Vec<GameProfileProperty>,
}

#[derive(Serialize, Deserialize)]
pub struct GameProfileTextures {
    pub(crate) textures: Textures,
}

#[derive(Serialize, Deserialize)]
pub struct Textures {
    #[serde(rename = "SKIN")]
    pub(crate) skin: Skin,
}

#[derive(Serialize, Deserialize)]
pub struct Skin {
    pub(crate) url: String,
    pub(crate) metadata: Option<SkinMetadata>,
}

#[derive(Serialize, Deserialize)]
pub struct SkinMetadata {
    pub(crate) model: String,
}

impl GameProfile {
    pub(crate) fn get_textures(&self) -> Result<GameProfileTextures> {
        let textures = self.properties.iter()
            .find(|property| property.name == "textures")
            .ok_or(NMSRaaSError::MissingTexturesProperty)?;

        let decoded = base64::decode(&textures.value).map_err(NMSRaaSError::Base64DecodeError)?;
        let decoded = String::from_utf8(decoded).map_err(|_|NMSRaaSError::MissingTexturesProperty)?;

        Ok(serde_json::from_str(&decoded).map_err(NMSRaaSError::InvalidJsonError)?)
    }
}


async fn get_player_game_profile(id: Uuid) -> Result<GameProfile> {
    reqwest::get(format!("https://sessionserver.mojang.com/session/minecraft/profile/{}", id))
        .await.map_err(NMSRaaSError::MojangRequestError)?
        .json::<GameProfile>()
        .await.map_err(NMSRaaSError::MojangRequestError)
}

pub(crate) async fn get_skin_hash(id: Uuid) -> Result<String> {
    let game_profile = get_player_game_profile(id).await?;
    let textures = game_profile.get_textures()?;
    let url = textures.textures.skin.url;

    // Take only after last slash
    Ok(get_skin_hash_from_url(url)?)
}

pub(crate) fn get_skin_hash_from_url(url: String) -> Result<String> {
    Ok(url.split('/').last().ok_or_else(|| NMSRaaSError::InvalidHashSkinUrl)?.to_string())
}

pub(crate) async fn get_skin_bytes(hash: String) -> Result<Bytes> {
    reqwest::get(format!("http://textures.minecraft.net/texture/{}", hash))
        .await.map_err(NMSRaaSError::MojangRequestError)?
        .bytes()
        .await.map_err(NMSRaaSError::MojangRequestError)
}