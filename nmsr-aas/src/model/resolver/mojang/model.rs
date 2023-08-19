use std::{
    borrow::Cow,
    collections::HashMap,
    marker::PhantomData,
    str::{self, FromStr},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use derive_more::{Debug, Deref};
use serde::{Deserialize, Deserializer};
use serde_json::{value::RawValue, Value};
use uuid::Uuid;

use crate::error::{MojangRequestError, MojangRequestResult};

#[derive(Deserialize, Debug)]
pub struct GameProfileTextureMetadata<'p> {
    model: &'p str,
}

impl GameProfileTextureMetadata<'_> {
    fn is_slim(&self) -> bool {
        self.model == "slim"
    }
}

#[derive(Deserialize, Debug)]
pub struct GameProfileTexture<'p> {
    url: &'p str,
    metadata: Option<GameProfileTextureMetadata<'p>>,
}

impl GameProfileTexture<'_> {
    pub fn is_slim(&self) -> bool {
        self.metadata.as_ref().map(|m| m.is_slim()).unwrap_or(false)
    }

    pub fn url(&self) -> &str {
        self.url
    }
}

#[derive(Deserialize, Debug)]
pub struct GameProfileTextures<'p> {
    #[serde(borrow)]
    textures: HashMap<&'p str, GameProfileTexture<'p>>,
}

impl<'p> GameProfileTextures<'p> {
    const SKIN_KEY: &'static str = "SKIN";
    const CAPE_KEY: &'static str = "CAPE";

    pub fn skin(&self) -> Option<&GameProfileTexture<'p>> {
        self.textures.get(Self::SKIN_KEY)
    }

    pub fn cape(&self) -> Option<&GameProfileTexture<'p>> {
        self.textures.get(Self::CAPE_KEY)
    }
}

#[derive(Deserialize)]
struct GameProfileProperty<'p> {
    name: &'p str,
    value: &'p [u8],
}

#[derive(Deserialize, Debug)]
pub struct GameProfile<'p> {
    id: Uuid,
    name: &'p str,
    #[serde(borrow, deserialize_with = "from_properties")]
    properties: HashMap<&'p str, Box<RawValue>>,
}

impl<'p> GameProfile<'p> {
    const TEXTURES_KEY: &'static str = "textures";

    fn textures(&'p self) -> MojangRequestResult<GameProfileTextures<'p>> {
        let textures = self
            .properties
            .get(Self::TEXTURES_KEY)
            .ok_or(MojangRequestError::MissingTexturesProperty)?;

        serde_json::from_str(textures.get())
            .map_err(|e| MojangRequestError::InvalidTexturesProperty(e))
    }
}

fn from_properties<'de: 'p, 'p, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<HashMap<&'p str, Box<RawValue>>, D::Error> {
    let value: Vec<GameProfileProperty<'p>> = Deserialize::deserialize(deserializer)?;
    let mut map = HashMap::new();

    for property in value {
        let decoded = STANDARD
            .decode(property.value)
            .map_err(|e| serde::de::Error::custom(e))?;

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

        println!("{:?}", profile);
        println!("{:?}", textures.skin());
        println!("{:?}", textures.cape());
    }
}
