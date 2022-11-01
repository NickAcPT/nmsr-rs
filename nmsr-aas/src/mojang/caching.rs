use crate::utils::Result;
use actix_web::web::Bytes;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct MojangCacheManager {
    root: PathBuf,
    skins: PathBuf,
    full_body_renders: PathBuf,
}

impl MojangCacheManager {
    pub(crate) fn init<P: AsRef<Path>>(root_path: P) -> Result<MojangCacheManager> {
        let root_path = root_path.as_ref().to_path_buf();
        let renders_path = root_path.join("renders");

        let skins_path = root_path.join("skins");
        let full_body_renders_path = renders_path.join("full_body");

        fs::create_dir_all(&root_path)?;
        fs::create_dir_all(&skins_path)?;
        fs::create_dir_all(&full_body_renders_path)?;

        Ok(MojangCacheManager {
            root: root_path,
            skins: skins_path,
            full_body_renders: full_body_renders_path,
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
}
