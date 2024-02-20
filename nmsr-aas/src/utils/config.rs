use std::{
    collections::HashMap,
    fs::Metadata,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Local};
use derive_more::Debug;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr, TryFromInto};
use tracing::trace;
use twelf::config;

use crate::{
    error::ExplainableExt,
    model::request::{
        cache::CacheBias, entry::RenderRequestEntry, RenderRequestFeatures, RenderRequestMode,
    },
};

#[config]
#[derive(Default, Debug)]
pub struct NmsrConfiguration {
    pub server: ServerConfiguration,
    pub tracing: Option<TracingConfiguration>,
    pub caching: ModelCacheConfiguration,
    pub mojank: MojankConfiguration,
    pub rendering: Option<RenderingConfiguration>,
    pub features: Option<FeaturesConfiguration>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ModelCacheConfiguration {
    /// The interval of time to run the cleanup task.
    /// This task will remove any files in the cache that are older than the image cache expiry.
    /// This task will run on startup, and then every time the interval has passed.
    #[serde(with = "humantime_serde")]
    pub cleanup_interval: Duration,

    /// The duration of time to keep a resolved model in the cache.
    /// This is effectively for how long to cache the UUID -> the player's skin, cape and other textures.
    /// When given a player uuid, we will resolve it with Mojang's API and cache the result.
    #[serde(with = "humantime_serde")]
    pub resolve_cache_duration: Duration,

    /// The duration of time to keep a texture in the cache.
    /// This is effectively for how long to cache the player's skin, cape and other textures
    /// even if the player's UUID wasn't requested for some time.
    #[serde(with = "humantime_serde")]
    pub texture_cache_duration: Duration,

    /// Cache biases for specific entries.
    /// A cache bias is a duration of time to keep a specific entry in the cache.
    /// This is useful for entries that are requested often, such as the models in the home page.
    #[serde_as(as = "HashMap<TryFromInto<String>, TryFromInto<String>>")]
    pub cache_biases: HashMap<RenderRequestEntry, CacheBias>,
}

impl Default for ModelCacheConfiguration {
    fn default() -> Self {
        Self {
            cleanup_interval: Duration::from_secs(60 * 60),
            resolve_cache_duration: Duration::from_secs(60 * 60 * 15),
            texture_cache_duration: Duration::from_secs(60 * 60 * 24 * 2),
            cache_biases: HashMap::new(),
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct MojankConfiguration {
    /// The session server to use for resolving player textures.
    /// This is used to resolve the player's skin, cape and other textures.
    pub session_server: String,

    /// The textures server to use for downloading player textures.
    pub textures_server: String,

    /// The GeyserMC API server to use for resolving Bedrock Edition player textures.
    pub geysermc_api_server: String,

    /// The rate limit to use for requests to the session server in a 1 second window.
    pub session_server_rate_limit: u64,
    
    /// Whether to enable the offline-mode UUIDs.
    /// By enabling this, offline mode UUIDs (version 3) will be allowed.
    pub allow_offline_mode_uuids: bool,
    
    /// Whether to use dashless UUIDs when requesting the Mojang API.
    /// By enabling this, the UUIDs will be sent to the Mojang API without dashes.
    pub use_dashless_uuids: bool,
    
    /// The template to use for resolving player skin textures.
    pub textures_server_skin_url_template: String,
    
    /// The template to use for resolving player cape textures.    
    pub textures_server_cape_url_template: String,
}

impl Default for MojankConfiguration {
    fn default() -> Self {
        Self {
            session_server: "https://sessionserver.mojang.com/".to_string(),
            textures_server: "https://textures.minecraft.net".to_string(),
            geysermc_api_server: "https://api.geysermc.org/".to_string(),
            session_server_rate_limit: 10,
            
            allow_offline_mode_uuids: false,
            use_dashless_uuids: false,
            textures_server_skin_url_template: "{textures_server}/texture/{texture_id}".to_string(),
            textures_server_cape_url_template: "{textures_server}/texture/{texture_id}".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerConfiguration {
    /// The address to bind the server to.
    pub address: String,
    /// The port to bind the server to.
    pub port: u16,
    /// The static files directory to serve.
    pub static_files_directory: Option<PathBuf>,
}

impl Default for ServerConfiguration {
    fn default() -> Self {
        Self {
            address: "0.0.0.0".to_string(),
            port: 8080,
            static_files_directory: None,
        }
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct TracingConfiguration {
    /// The OpenTelemetry endpoint to send traces to.
    pub endpoint: String,
    /// The service name to use for traces.
    #[serde(default = "default_service_name")]
    pub service_name: String,
}

#[derive(Default, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct RenderingConfiguration {
    /// The number of MSAA samples to use when rendering.
    pub sample_count: u32,
    /// Whether to use SMAA.
    pub use_smaa: bool,
}

#[serde_as]
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct FeaturesConfiguration {
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub disabled_features: Vec<RenderRequestFeatures>,

    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub disabled_modes: Vec<RenderRequestMode>,
}

impl ModelCacheConfiguration {
    #[must_use]
    pub fn get_cache_duration(&self, entry: &RenderRequestEntry) -> &Duration {
        self.get_cache_duration_with_default(entry, &self.resolve_cache_duration)
    }

    #[must_use]
    pub fn get_cache_duration_with_default<'a>(
        &'a self,
        entry: &RenderRequestEntry,
        default_duration: &'a Duration,
    ) -> &'a Duration {
        let bias = self.cache_biases.get(entry);

        bias.map_or(default_duration, |bias| {
            trace!("Found cache bias for entry: {:?}", bias);

            match bias {
                CacheBias::KeepCachedFor(duration) => duration,
                CacheBias::CacheIndefinitely => &Duration::MAX,
            }
        })
    }

    pub fn is_expired(
        &self,
        entry: &RenderRequestEntry,
        marker_metadata: &Metadata,
    ) -> crate::error::Result<bool> {
        self.is_expired_with_default(entry, marker_metadata, &self.resolve_cache_duration)
    }

    pub fn is_expired_with_default(
        &self,
        entry: &RenderRequestEntry,
        marker_metadata: &Metadata,
        default_duration: &Duration,
    ) -> crate::error::Result<bool> {
        let duration = self.get_cache_duration_with_default(entry, default_duration);

        // Short-circuit never expiring entry.
        if duration == &Duration::MAX {
            return Ok(false);
        }

        let expiry = marker_metadata.modified().explain(format!(
            "Unable to get marker modified date for entry {:?}",
            &entry
        ))? + *duration;

        trace!("Entry expires on {}", Into::<DateTime<Local>>::into(expiry));

        Ok(expiry < SystemTime::now())
    }

    const VALID_PNG_HEADER: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    #[must_use]
    pub fn validate_png_data(&self, data: &[u8]) -> bool {
        // Cheeky hack to validate that the texture is valid
        let data_header = data.get(0..Self::VALID_PNG_HEADER.len());

        if let Some(data_header) = data_header {
            data_header == Self::VALID_PNG_HEADER
        } else {
            false
        }
    }
}

fn default_service_name() -> String {
    "nmsr-aas".to_string()
}
