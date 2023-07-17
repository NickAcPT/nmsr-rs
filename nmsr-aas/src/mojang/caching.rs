use std::collections::HashMap;
use std::fs;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use actix_web::web::Bytes;
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use strum::IntoEnumIterator;
use tracing::debug;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::manager::RenderMode;
use crate::mojang::requests::CachedSkinHash;
use crate::routes::render_body_route::{RenderDataCacheKey};
use crate::utils::Result;

pub(crate) type RateLimiterType = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

#[derive(Debug, Clone)]
struct CachedUuidToSkinHash {
    time: Instant,
    value: CachedSkinHash
}

#[derive(Debug)]
pub(crate) struct MojangCacheManager {
    root: PathBuf,
    skins: PathBuf,
    renders_dir: PathBuf,
    resolved_uuid_to_skin_hash_cache: HashMap<Uuid, CachedUuidToSkinHash>,

    renders_and_skin_cache_expiry: Duration,
    uuid_to_skin_hash_cache_expiry: Duration,

    pub(crate) rate_limiter: Arc<RateLimiterType>,
}

impl MojangCacheManager {
    pub(crate) fn cleanup_old_files(&self) -> Result<()> {
        let now = std::time::SystemTime::now();

        for entry in WalkDir::new(&self.root) {
            let entry = entry?;
            if !entry.path().is_file() {
                continue;
            }
            let modified = entry.metadata()?.modified()?;
            if now.duration_since(modified)? > self.renders_and_skin_cache_expiry {
                fs::remove_file(entry.path())?;
            }
        }

        Ok(())
    }

    pub(crate) fn init<P: AsRef<Path>>(
        root_path: P,
        renders_and_skin_cache_expiry: u32,
        uuid_to_skin_hash_cache_expiry: u32,
        mojang_api_rate_limit: u32,
    ) -> Result<MojangCacheManager> {
        let root_path = root_path.as_ref().to_path_buf();
        let renders_path = root_path.join("renders");

        let skins_path = root_path.join("skins");

        let rate_limiter = RateLimiter::direct(Quota::per_second(
            NonZeroU32::new(mojang_api_rate_limit).expect("mojang_api_rate_limit must be > 0"),
        ));

        fs::create_dir_all(&root_path)?;
        fs::create_dir_all(&skins_path)?;
        fs::create_dir_all(&renders_path)?;

        let manager = MojangCacheManager {
            root: root_path,
            skins: skins_path,
            renders_dir: renders_path,
            resolved_uuid_to_skin_hash_cache: HashMap::new(),

            renders_and_skin_cache_expiry: Duration::from_secs(
                renders_and_skin_cache_expiry as u64,
            ),
            uuid_to_skin_hash_cache_expiry: Duration::from_secs(
                uuid_to_skin_hash_cache_expiry as u64,
            ),
            rate_limiter: Arc::new(rate_limiter),
        };

        for mode in RenderMode::iter() {
            fs::create_dir_all(manager.get_cached_render_mode_path(&mode))?;
        }

        Ok(manager)
    }

    fn get_cached_skin_path(&self, hash: &String) -> PathBuf {
        self.skins.join(hash)
    }

    fn get_cached_render_path(&self, mode: &RenderMode, hash: &String, render_data: &RenderDataCacheKey) -> PathBuf {
        self.get_cached_render_mode_path(mode).join(format!(
            "{}_{}_{}_{}.png",
            hash,
            render_data.slim_arms,
            render_data.include_layers,
            render_data.include_shading
        ))
    }

    fn get_cached_render_mode_path(&self, mode: &RenderMode) -> PathBuf {
        self.renders_dir.join(mode.to_string())
    }

    pub(crate) fn get_cached_skin(&self, hash: &String) -> Result<Option<Vec<u8>>> {
        debug!("Getting cached skin for hash {}", hash);
        let path = self.get_cached_skin_path(hash);
        if path.exists() {
            debug!("Found cached skin for hash {}", hash);
            Ok(Some(fs::read(path)?))
        } else {
            debug!("No cached skin for hash {}", hash);
            Ok(None)
        }
    }

    pub(crate) fn cache_skin(&self, hash: &String, bytes: &Bytes) -> Result<()> {
        let path = self.get_cached_skin_path(hash);
        fs::write(path, bytes)?;
        Ok(())
    }

    pub(crate) fn get_cached_render(
        &self,
        mode: &RenderMode,
        hash: &String,
        render_data: &RenderDataCacheKey,
    ) -> Result<Option<Vec<u8>>> {
        debug!(
            "Getting cached render for hash {} in mode {}, render_data: {:?}",
            hash,
            mode.to_string(),
            render_data
        );
        let path = self.get_cached_render_path(mode, hash, render_data);
        if path.exists() {
            debug!("Found cached render for hash {}", hash);
            Ok(Some(fs::read(path)?))
        } else {
            debug!("No cached render for hash {}", hash);
            Ok(None)
        }
    }

    pub(crate) fn cache_render(
        &self,
        mode: &RenderMode,
        hash: &String,
        render_data: &RenderDataCacheKey,
        bytes: &[u8],
    ) -> Result<()> {
        let path = self.get_cached_render_path(mode, hash, render_data);
        fs::write(path, bytes)?;
        Ok(())
    }

    pub(crate) fn get_cached_uuid_to_skin_hash(&self, uuid: &Uuid) -> Option<&CachedSkinHash> {
        debug!("Checking cache for {}", uuid);
        if let Some(cached) = self.resolved_uuid_to_skin_hash_cache.get(uuid) {
            return if Self::is_cached_uuid_to_skin_hash_expired(cached, self.uuid_to_skin_hash_cache_expiry) {
                debug!("Found cached hash for {}", uuid);
                Some(&cached.value)
            } else {
                debug!("Cached hash for {} expired", uuid);
                None
            };
        }
        debug!("No cached hash for {}", uuid);
        None
    }

    pub(crate) fn cache_uuid_to_skin_hash_and_model(&mut self, uuid: &Uuid, data: CachedSkinHash) {

        self.resolved_uuid_to_skin_hash_cache.insert(
            *uuid,
            CachedUuidToSkinHash {
                time: Instant::now(),
                value: data
            },
        );
    }

    fn is_cached_uuid_to_skin_hash_expired(cache: &CachedUuidToSkinHash, cache_duration: Duration) -> bool {
        cache.time.elapsed() < cache_duration
    }

    pub(crate) fn purge_expired_uuid_to_skin_hash_cache(&mut self) {
        self.resolved_uuid_to_skin_hash_cache.retain(|_, cache| Self::is_cached_uuid_to_skin_hash_expired(cache, self.uuid_to_skin_hash_cache_expiry));
    }
}
