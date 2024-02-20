use crate::error::{MojangRequestError, MojangRequestResult};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct GameProfileTextureMetadata {
    model: String,
}

impl GameProfileTextureMetadata {
    fn is_slim(&self) -> bool {
        self.model == "slim"
    }
}

#[derive(Deserialize, Debug)]
pub struct GameProfileTexture {
    url: String,
    metadata: Option<GameProfileTextureMetadata>,
}

impl GameProfileTexture {
    #[must_use]
    pub fn is_slim(&self) -> bool {
        self.metadata
            .as_ref()
            .is_some_and(GameProfileTextureMetadata::is_slim)
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn hash(&self) -> MojangRequestResult<&str> {
        self.url
            .split('/')
            .last()
            .map(|s| s.strip_suffix(".png").unwrap_or(s))
            .ok_or_else(|| MojangRequestError::InvalidTextureUrlError(self.url.clone()))
    }
}

#[derive(Deserialize, Debug)]
pub struct GameProfileTextures {
    textures: HashMap<String, GameProfileTexture>,
}

impl GameProfileTextures {
    const SKIN_KEY: &'static str = "SKIN";
    const CAPE_KEY: &'static str = "CAPE";

    #[must_use]
    pub fn skin(&self) -> Option<&GameProfileTexture> {
        self.textures.get(Self::SKIN_KEY)
    }

    #[must_use]
    pub fn cape(&self) -> Option<&GameProfileTexture> {
        self.textures.get(Self::CAPE_KEY)
    }
}

#[derive(Deserialize)]
struct GameProfileProperty {
    name: String,
    value: String,
}

#[derive(Deserialize, Debug)]
pub struct GameProfile {
    #[serde(deserialize_with = "from_properties")]
    properties: HashMap<String, Value>,
}

impl GameProfile {
    const TEXTURES_KEY: &'static str = "textures";

    pub fn textures(&self) -> MojangRequestResult<GameProfileTextures> {
        let textures = self
            .properties
            .get(Self::TEXTURES_KEY)
            .ok_or(MojangRequestError::MissingTexturesPropertyError)?;

        serde_json::from_value(textures.clone())
            .map_err(MojangRequestError::InvalidTexturesPropertyError)
    }
}

fn from_properties<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<HashMap<String, Value>, D::Error> {
    let value: Vec<GameProfileProperty> = Deserialize::deserialize(deserializer)?;
    let mut map = HashMap::new();

    for property in value {
        let decoded = STANDARD
            .decode(property.value)
            .map_err(serde::de::Error::custom)?;

        let value = serde_json::from_slice(&decoded).unwrap();

        map.insert(property.name, value);
    }

    Ok(map)
}

#[cfg(test)]
pub mod test {

    #[test]
    fn owo() {
        let input = r#"{
            "id" : "4566e69fc90748ee8d71d7ba5aa00d20",
            "name" : "Thinkofdeath",
            "properties" : [ {
              "name" : "textures",
              "value" : "ewogICJ0aW1lc3RhbXAiIDogMTY5MjM1NDc4OTc1NCwKICAicHJvZmlsZUlkIiA6ICI0NTY2ZTY5ZmM5MDc0OGVlOGQ3MWQ3YmE1YWEwMGQyMCIsCiAgInByb2ZpbGVOYW1lIiA6ICJUaGlua29mZGVhdGgiLAogICJ0ZXh0dXJlcyIgOiB7CiAgICAiU0tJTiIgOiB7CiAgICAgICJ1cmwiIDogImh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvNzRkMWUwOGIwYmI3ZTlmNTkwYWYyNzc1ODEyNWJiZWQxNzc4YWM2Y2VmNzI5YWVkZmNiOTYxM2U5OTExYWU3NSIKICAgIH0sCiAgICAiQ0FQRSIgOiB7CiAgICAgICJ1cmwiIDogImh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvYjBjYzA4ODQwNzAwNDQ3MzIyZDk1M2EwMmI5NjVmMWQ2NWExM2E2MDNiZjY0YjE3YzgwM2MyMTQ0NmZlMTYzNSIKICAgIH0KICB9Cn0="
            } ],
            "profileActions" : [ ]
          }"#;

        let profile: super::GameProfile = serde_json::from_str(input).unwrap();

        let textures = profile.textures().unwrap();

        println!("{profile:?}");
        println!("{:?}", textures.skin());
        println!("{:?}", textures.cape());
    }
}
