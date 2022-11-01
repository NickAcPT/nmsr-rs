use crate::manager::RenderMode;
use crate::utils::Result;
use actix_web::web::Bytes;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct CachedUuidToSkinHash {
    time: Instant,
    hash: String,
}

#[derive(Debug, Clone)]
pub(crate) struct MojangCacheManager {
    root: PathBuf,
    skins: PathBuf,
    renders_dir: PathBuf,
    resolved_uuid_to_skin_hash_cache: HashMap<Uuid, CachedUuidToSkinHash>,
}

const SKIN_CACHE_EXPIRE: Duration = Duration::from_secs(60 * 60 * 24 * 7); // 7 days ( 60 seconds * 60 minutes * 24 hours * 7 days )
const UUID_TO_SKIN_HASH_CACHE_EXPIRE: Duration = Duration::from_secs(60); // 1 minute ( 60 seconds )

impl MojangCacheManager {
    pub(crate) fn cleanup_old_files(&self) -> Result<()> {
        let now = std::time::SystemTime::now();

        for file in WalkDir::new(&self.root) {
            let file = file?;
            let modified = file.metadata()?.modified()?;
            if now.duration_since(modified)? > SKIN_CACHE_EXPIRE {
                fs::remove_file(file.path())?;
            }
        }

        Ok(())
    }

    pub(crate) fn init<P: AsRef<Path>>(root_path: P) -> Result<MojangCacheManager> {
        let root_path = root_path.as_ref().to_path_buf();
        let renders_path = root_path.join("renders");

        let skins_path = root_path.join("skins");

        fs::create_dir_all(&root_path)?;
        fs::create_dir_all(&skins_path)?;
        fs::create_dir_all(&renders_path)?;

        let manager = MojangCacheManager {
            root: root_path,
            skins: skins_path,
            renders_dir: renders_path,
            resolved_uuid_to_skin_hash_cache: HashMap::new(),
        };

        for mode in RenderMode::iter() {
            fs::create_dir_all(manager.get_cached_render_mode_path(&mode))?;
        }

        Ok(manager)
    }

    fn get_cached_skin_path(&self, hash: &String) -> PathBuf {
        self.skins.join(hash)
    }

    fn get_cached_render_path(&self, mode: &RenderMode, hash: &String, slim_arms: bool) -> PathBuf {
        self.get_cached_render_mode_path(mode).join(format!(
            "{}_{}.png",
            hash,
            if slim_arms { "slim" } else { "classic" },
        ))
    }

    fn get_cached_render_mode_path(&self, mode: &RenderMode) -> PathBuf {
        self.renders_dir.join(mode.to_string())
    }

    pub(crate) fn get_cached_skin(&self, hash: &String) -> Result<Option<Vec<u8>>> {
        let path = self.get_cached_skin_path(hash);
        if path.exists() {
            Ok(Some(fs::read(path)?))
        } else {
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
        slim_arms: bool,
    ) -> Result<Option<Vec<u8>>> {
        let path = self.get_cached_render_path(mode, hash, slim_arms);
        if path.exists() {
            Ok(Some(fs::read(path)?))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn cache_render(
        &self,
        mode: &RenderMode,
        hash: &String,
        slim_arms: bool,
        bytes: &[u8],
    ) -> Result<()> {
        let path = self.get_cached_render_path(mode, hash, slim_arms);
        fs::write(path, bytes)?;
        Ok(())
    }

    pub(crate) fn get_cached_uuid_to_skin_hash(&mut self, uuid: &Uuid) -> Option<String> {
        if let Some(cached) = self.resolved_uuid_to_skin_hash_cache.get(uuid) {
            return if cached.time.elapsed() < UUID_TO_SKIN_HASH_CACHE_EXPIRE {
                Some(cached.hash.clone())
            } else {
                self.resolved_uuid_to_skin_hash_cache.remove(uuid);
                None
            };
        }
        None
    }

    pub(crate) fn cache_uuid_to_skin_hash(&mut self, uuid: &Uuid, hash: &str) {
        self.resolved_uuid_to_skin_hash_cache.insert(
            *uuid,
            CachedUuidToSkinHash {
                time: Instant::now(),
                hash: hash.to_owned(),
            },
        );
    }
}
