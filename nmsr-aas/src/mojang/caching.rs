use crate::utils::Result;
use actix_web::web::Bytes;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone)]
struct CachedUuidToSkinHash {
    time: Instant,
    hash: String,
}

#[derive(Debug, Clone)]
pub(crate) struct MojangCacheManager {
    skins: PathBuf,
    full_body_renders: PathBuf,
    resolved_uuid_to_skin_hash_cache: HashMap<Uuid, CachedUuidToSkinHash>,
}

const SKIN_CACHE_EXPIRE: Duration = Duration::from_secs(60 * 60 * 24 * 7); // 7 days ( 60 seconds * 60 minutes * 24 hours * 7 days )
const UUID_TO_SKIN_HASH_CACHE_EXPIRE: Duration = Duration::from_secs(60); // 1 minute ( 60 seconds )

impl MojangCacheManager {
    pub(crate) fn cleanup_old_files(&self) -> Result<()> {
        let files = fs::read_dir(&self.skins)?.chain(fs::read_dir(&self.full_body_renders)?);
        let now = std::time::SystemTime::now();

        for file in files {
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
        let full_body_renders_path = renders_path.join("full_body");

        fs::create_dir_all(&root_path)?;
        fs::create_dir_all(&skins_path)?;
        fs::create_dir_all(&full_body_renders_path)?;

        Ok(MojangCacheManager {
            skins: skins_path,
            full_body_renders: full_body_renders_path,
            resolved_uuid_to_skin_hash_cache: HashMap::new(),
        })
    }

    fn get_cached_skin_path(&self, hash: &String) -> PathBuf {
        self.skins.join(hash)
    }

    fn get_cached_full_body_render_path(&self, hash: &String) -> PathBuf {
        self.full_body_renders.join(hash)
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

    pub(crate) fn get_cached_full_body_render(&self, hash: &String) -> Result<Option<Vec<u8>>> {
        let path = self.get_cached_full_body_render_path(hash);
        if path.exists() {
            Ok(Some(fs::read(path)?))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn cache_full_body_render(&self, hash: &String, bytes: &[u8]) -> Result<()> {
        let path = self.get_cached_full_body_render_path(hash);
        fs::write(path, bytes)?;
        Ok(())
    }

    pub(crate) fn get_cached_uuid_to_skin_hash(&mut self, uuid: &Uuid) -> Option<String> {
        if let Some(cached) = self.resolved_uuid_to_skin_hash_cache.get(uuid) {
            if cached.time.elapsed() < UUID_TO_SKIN_HASH_CACHE_EXPIRE {
                return Some(cached.hash.clone());
            } else {
                self.resolved_uuid_to_skin_hash_cache.remove(uuid);
            }
        }
        None
    }

    pub(crate) fn cache_uuid_to_skin_hash(&mut self, uuid: &Uuid, hash: &String) {
        self.resolved_uuid_to_skin_hash_cache.insert(
            *uuid,
            CachedUuidToSkinHash {
                time: Instant::now(),
                hash: hash.clone(),
            },
        );
    }
}
