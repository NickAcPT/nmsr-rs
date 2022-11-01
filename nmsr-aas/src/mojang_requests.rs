use crate::utils::errors::NMSRaaSError;
use crate::utils::Result;
use actix_web::web::Bytes;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct GameProfileProperty {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct GameProfile {
    pub id: String,
    pub name: String,
    pub properties: Vec<GameProfileProperty>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameProfileTextures {
    pub(crate) textures: Textures,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Textures {
    #[serde(rename = "SKIN")]
    pub(crate) skin: Skin,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Skin {
    pub(crate) url: String,
    pub(crate) metadata: Option<SkinMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkinMetadata {
    pub(crate) model: String,
}

impl GameProfile {
    pub(crate) fn get_textures(&self) -> Result<GameProfileTextures> {
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

async fn get_player_game_profile(client: &reqwest::Client, id: Uuid) -> Result<GameProfile> {
    Ok(client
        .get(format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{}",
            id
        ))
        .send()
        .await?
        .json::<GameProfile>()
        .await?)
}

pub(crate) async fn get_skin_hash(client: &reqwest::Client, id: Uuid) -> Result<String> {
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

pub(crate) async fn get_skin_bytes(hash: String) -> Result<Bytes> {
    // Check if file at path "cache/skin/hash" exists
    // If it does, return the contents of the file

    let path = format!("cache/skin/{}", hash);
    if Path::new(&path).exists() {
        let mut file = File::open(path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(NMSRaaSError::IOError)?;
        return Ok(Bytes::from(contents));
    }

    let bytes = reqwest::get(format!("http://textures.minecraft.net/texture/{}", hash))
        .await?
        .bytes()
        .await?;

    // Write bytes to file at path "cache/skin/hash"
    let mut file = File::create(format!("cache/skin/{}", hash)).map_err(NMSRaaSError::IOError)?;
    file.write_all(&bytes).map_err(NMSRaaSError::IOError)?;

    Ok(bytes)
}
