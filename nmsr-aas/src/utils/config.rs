use std::{collections::HashMap, time::{Duration, SystemTime}, fs::Metadata};

use serde::{Deserialize, Serialize};
use twelf::config;

use crate::{model::request::{cache::CacheBias, entry::RenderRequestEntry}, error::ExplainableExt};

#[config]
#[derive(Default)]
pub struct Configuration {
    pub caching: ModelCacheConfiguration,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
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

impl ModelCacheConfiguration {
    pub fn is_expired(&self, entry: &RenderRequestEntry, marker_metadata: Metadata, default_duration: &Duration) -> crate::error::Result<bool> {
        let bias = self.cache_biases.get(entry);

        let duration = if let Some(bias) = bias {
            match bias {
                CacheBias::KeepCachedFor(duration) => duration,
                CacheBias::CacheIndefinitely => &Duration::MAX,
            }
        } else {
            default_duration
        };

        // Short-circuit never expiring entry.
        if duration == &Duration::MAX {
            return Ok(false);
        }

        let expiry = marker_metadata.modified().explain(format!(
            "Unable to get marker modified date for entry {:?}",
            &entry
        ))? + *duration;

        return Ok(expiry < SystemTime::now());
    }
}