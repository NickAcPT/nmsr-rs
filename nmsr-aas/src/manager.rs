use std::borrow::Cow;
#[cfg(not(feature = "lazy_parts"))]
use std::collections::HashMap;
use std::path::Path;

use strum::IntoEnumIterator;
use strum::{Display, EnumCount, EnumIter, EnumString};
use tracing::{debug, instrument};

use nmsr_lib::parts::manager::PartsManager;
use nmsr_lib::vfs::{PhysicalFS, VfsPath};
#[cfg(feature = "lazy_parts")]
use {
    rayon::prelude::*,
    std::io::{BufReader, BufWriter, Write},
    crate::utils::errors::NMSRaaSError,
};

use crate::utils::errors::NMSRaaSError::MissingPartManager;
use crate::utils::Result;

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString, EnumIter, EnumCount, Display)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum RenderMode {
    FullBody,
    FrontFull,
    FullBodyIso,
    Head,
    HeadIso,
    Face,
}

#[derive(Debug, Clone)]
pub(crate) struct NMSRaaSManager {
    #[cfg(feature = "lazy_parts")]
    part_root: VfsPath,
    #[cfg(not(feature = "lazy_parts"))]
    part_managers: HashMap<RenderMode, PartsManager>,
}

impl NMSRaaSManager {
    #[instrument(skip(part_root))]
    fn create_part_manager_for_mode(
        part_root: &VfsPath,
        render_type: &RenderMode,
    ) -> Result<PartsManager> {
        let path = part_root.join(render_type.to_string())?;

        Ok(PartsManager::new(&path)?)
    }
}

#[cfg(not(feature = "lazy_parts"))]
impl NMSRaaSManager {
    pub(crate) fn get_manager(&self, render_type: &RenderMode) -> Result<Cow<PartsManager>> {
        self.part_managers
            .get(render_type)
            .map(Cow::Borrowed)
            .ok_or_else(|| MissingPartManager(render_type.clone()))
    }

    #[instrument(skip(part_root))]
    pub(crate) fn new(part_root: impl AsRef<Path>) -> Result<NMSRaaSManager> {
        let part_root: VfsPath = PhysicalFS::new(part_root).into();
        let mut map = HashMap::with_capacity(RenderMode::COUNT);

        for render_type in RenderMode::iter() {
            let manager = Self::create_part_manager_for_mode(&part_root, &render_type)?;
            map.insert(render_type, manager);
        }

        Ok(NMSRaaSManager { part_managers: map })
    }
}

#[cfg(feature = "lazy_parts")]
impl NMSRaaSManager {
    pub(crate) fn get_manager(&self, render_type: &RenderMode) -> Result<Cow<PartsManager>> {
        let lazy_parts_dir = Self::get_lazy_parts_directory(&self.part_root)?;
        let part_path = Self::get_render_mode_part_manager_path(&lazy_parts_dir, render_type)?;

        if part_path.exists()? {
            let reader = BufReader::new(part_path.open_file()?);

            let start = std::time::Instant::now();
            let manager = bincode::deserialize_from(reader)?;
            debug!(
                "Deserialized part manager for {:?} in {:?}",
                render_type,
                start.elapsed()
            );

            Ok(Cow::Owned(manager))
        } else {
            Err(MissingPartManager(render_type.clone()))
        }
    }

    fn get_lazy_parts_directory(part_root: &VfsPath) -> Result<VfsPath> {
        Ok(part_root.join("lazy_parts")?)
    }

    fn get_render_mode_part_manager_path(
        lazy_parts_dir: &VfsPath,
        render_type: &RenderMode,
    ) -> Result<VfsPath> {
        Ok(lazy_parts_dir.join(render_type.to_string())?)
    }

    #[instrument(skip(part_root))]
    pub(crate) fn new(part_root: impl AsRef<Path>) -> Result<NMSRaaSManager> {
        let part_root = PhysicalFS::new(part_root).into();
        let lazy_parts_dir = Self::get_lazy_parts_directory(&part_root)?;

        // Yeet all the old parts we made just in case.
        // It's a one time action so it's fine™
        lazy_parts_dir.remove_dir_all()?;
        lazy_parts_dir.create_dir_all()?;

        let serialized_parts: Vec<_> = RenderMode::iter()
            .par_bridge()
            .map(|render_type| {
                let manager = Self::create_part_manager_for_mode(&part_root, &render_type);
                let serialized = manager.and_then(|manager| {
                    bincode::serialize(&manager).map_err(NMSRaaSError::BincodeError)
                });

                (render_type, serialized)
            })
            .collect();

        for (mode, serialized_part) in serialized_parts {
            let file = Self::get_render_mode_part_manager_path(&lazy_parts_dir, &mode)?;
            let mut writer = BufWriter::new(file.create_file()?);
            let data = serialized_part?;

            writer.write_all(data.as_slice())?;
        }

        Ok(NMSRaaSManager { part_root })
    }
}
