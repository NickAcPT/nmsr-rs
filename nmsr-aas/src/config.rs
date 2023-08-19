use std::{time::Duration, collections::HashMap};

use serde::{Deserialize, Serialize};
use twelf::config;

use crate::model::request::{entry::RenderRequestEntry, cache::CacheBias};

#[config]
#[derive(Default)]
pub struct Configuration {
    pub caching: ModelCacheConfiguration    
}

#[derive(Default, Serialize, Deserialize)]
pub struct ModelCacheConfiguration {
    /// The duration of time to keep a resolved model in the cache.
    /// This is effectively for how long to cache the player's skin, cape and other textures.
    /// When given a player uuid, we will resolve it with Mojang's API and cache the result.
    #[serde(with = "humantime_serde")]
    pub(crate) resolve_cache_duration: Duration,

    /// The duration of time to keep a rendered model in the cache.
    /// This is effectively for how long to cache the rendered outputs.
    #[serde(with = "humantime_serde")]
    pub(crate) render_cache_duration: Duration,
    
    /// Cache biases for specific entries.
    /// A cache bias is a duration of time to keep a specific entry in the cache.
    /// This is useful for entries that are requested often, such as the models in the home page.
    pub(crate) cache_biases: HashMap<RenderRequestEntry, CacheBias>,
}