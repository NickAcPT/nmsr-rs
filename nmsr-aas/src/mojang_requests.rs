use crate::utils::errors::NMSRaaSError;
use crate::utils::Result;
use actix_web::web::Bytes;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;

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
        let textures = self
            .properties
            .iter()
            .find(|property| property.name == "textures")
            .ok_or(NMSRaaSError::MissingTexturesProperty)?;

        let decoded = base64::decode(&textures.value).map_err(NMSRaaSError::Base64DecodeError)?;
        let decoded =
            String::from_utf8(decoded).map_err(|_| NMSRaaSError::MissingTexturesProperty)?;

        serde_json::from_str(&decoded).map_err(NMSRaaSError::InvalidJsonError)
    }
}

async fn get_player_game_profile(id: Uuid) -> Result<GameProfile> {
    reqwest::get(format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{}",
        id
    ))
    .await
    .map_err(NMSRaaSError::MojangRequestError)?
    .json::<GameProfile>()
    .await
    .map_err(NMSRaaSError::MojangRequestError)
}

pub(crate) async fn get_skin_hash(id: Uuid) -> Result<String> {
    // Check if file at path "cache/uuid/id" exists
    // If it does, return the contents of the file

    let path = format!("cache/uuid/{}", id);
    if Path::new(&path).exists() {
        let mut file = File::open(path).map_err(NMSRaaSError::IOError)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(NMSRaaSError::IOError)?;
        return Ok(contents);
    }

    let game_profile = get_player_game_profile(id).await?;
    let textures = game_profile.get_textures()?;
    let url = textures.textures.skin.url;

    // Take only after last slash
    let hash = get_skin_hash_from_url(url)?;

    // Write hash to file at path "cache/uuid/id"
    let mut file = File::create(format!("cache/uuid/{}", id)).map_err(NMSRaaSError::IOError)?;
    file.write_all(hash.as_bytes())
        .map_err(NMSRaaSError::IOError)?;

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
        let mut file = File::open(path).map_err(NMSRaaSError::IOError)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(NMSRaaSError::IOError)?;
        return Ok(Bytes::from(contents));
    }

    let bytes = reqwest::get(format!("http://textures.minecraft.net/texture/{}", hash))
        .await
        .map_err(NMSRaaSError::MojangRequestError)?
        .bytes()
        .await
        .map_err(NMSRaaSError::MojangRequestError)?;

    // Write bytes to file at path "cache/skin/hash"
    let mut file = File::create(format!("cache/skin/{}", hash)).map_err(NMSRaaSError::IOError)?;
    file.write_all(&bytes).map_err(NMSRaaSError::IOError)?;

    Ok(bytes)
}
