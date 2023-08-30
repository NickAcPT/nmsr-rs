use std::{collections::HashMap, time::{Duration, SystemTime}, fs::Metadata};

use chrono::{DateTime, Local};
use derive_more::Debug;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TryFromInto};
use tracing::{instrument, trace};
use twelf::config;

use crate::{model::request::{cache::CacheBias, entry::RenderRequestEntry}, error::ExplainableExt};

#[config]
#[derive(Default, Debug)]
pub struct NmsrConfiguration {
    pub server: ServerConfiguration,
    pub tracing: Option<TracingConfiguration>,
    pub caching: ModelCacheConfiguration,
    pub mojank: MojankConfiguration,
}

#[serde_as]
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct ModelCacheConfiguration {
    /// The interval of time to run the cleanup task.
    /// This task will remove any files in the cache that are older than the image cache expiry.
    /// This task will run on startup, and then every time the interval has passed.
    #[serde(with = "humantime_serde")]
    pub cleanup_interval: Duration,
    
    /// The duration of time to keep a resolved model in the cache.
    /// This is effectively for how long to cache the player's skin, cape and other textures.
    /// When given a player uuid, we will resolve it with Mojang's API and cache the result.
    #[serde(with = "humantime_serde")]
    pub resolve_cache_duration: Duration,

    /// The duration of time to keep a rendered model in the cache.
    /// This is effectively for how long to cache the rendered outputs.
    #[serde(with = "humantime_serde")]
    pub render_cache_duration: Duration,

    /// Cache biases for specific entries.
    /// A cache bias is a duration of time to keep a specific entry in the cache.
    /// This is useful for entries that are requested often, such as the models in the home page.
    #[serde_as(as = "HashMap<TryFromInto<String>, TryFromInto<String>>")]
    pub cache_biases: HashMap<RenderRequestEntry, CacheBias>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct MojankConfiguration {
    /// The session server to use for resolving player textures.
    /// This is used to resolve the player's skin, cape and other textures.
    #[serde(default = "default_session_server")]
    pub session_server: String,
    
    /// The textures server to use for downloading player textures.
    #[serde(default = "default_textures_server")]
    pub textures_server: String,
    
    /// The rate limit to use for requests to the session server in a 1 second window.
    #[serde(default = "default_session_server_rate_limit")]
    pub session_server_rate_limit: u64,
}

#[inline]
fn default_session_server_rate_limit() -> u64 {
    100
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct ServerConfiguration {
    /// The address to bind the server to.
    pub address: String,
    /// The port to bind the server to.
    pub port: u16,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct TracingConfiguration {
    /// The OpenTelemetry endpoint to send traces to.
    pub endpoint: String,
    /// The service name to use for traces.
    #[serde(default = "default_service_name")]
    pub service_name: String,
}

impl ModelCacheConfiguration {
    pub fn is_expired(&self, entry: &RenderRequestEntry, marker_metadata: Metadata, default_duration: &Duration) -> crate::error::Result<bool> {
        let bias = self.cache_biases.get(entry);

        let duration = if let Some(bias) = bias {
            trace!("Found cache bias for entry: {:?}", bias);
            
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
        
        trace!("Entry expires on {}", Into::<DateTime<Local>>::into(expiry));
        
        return Ok(expiry < SystemTime::now());
    }
}

fn default_service_name() -> String {
    "nmsr-aas".to_string()
}

fn default_session_server() -> String {
    "https://sessionserver.mojang.com/".to_string()
}

fn default_textures_server() -> String {
    "https://textures.minecraft.net".to_string()
}