use std::borrow::Borrow;
use std::collections::hash_map::Iter;
use std::iter::Filter;
use crate::parts::manager::PartsManager;
use crate::rendering::entry::RenderingEntry;
use crate::uv::uv_magic::UvImage;

impl PartsManager {
    pub(crate) fn get_parts(&self, entry: &RenderingEntry) -> Vec<&UvImage> {
        let required_parts = self.all_parts.iter();

        let model_parts = self
            .model_parts
            .iter()
            .filter(|(key, _)| key.starts_with(entry.model.get_dir_name()));

        required_parts
            .chain(model_parts)
            .map(|(_, uv)| uv)
            .collect()
    }

    pub(crate) fn get_overlays(&self, entry: &RenderingEntry, uv: &UvImage) -> Vec<&UvImage> {
        let layer_name = if uv.name.contains("Layer") { "layer" } else { "base" };
        let model_name = entry.model.borrow().get_dir_name();

        self.model_overlays.iter()
            .filter(|(key, _)| key.starts_with(model_name) && key.contains(layer_name))
            .map(|(_, uv)| uv)
            .collect()
    }
}
