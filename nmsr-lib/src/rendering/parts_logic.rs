use crate::parts::manager::PartsManager;
use crate::rendering::entry::RenderingEntry;
use crate::uv::uv_magic::UvImage;

impl PartsManager {
    pub(crate) fn get_parts(&self, entry: &RenderingEntry) -> Vec<&UvImage> {
        let required_parts = self.all_parts.iter();

        let model_parts = self
            .model_parts
            .iter()
            .filter(|uv| uv.name.starts_with(entry.model.get_dir_name()));

        required_parts
            .chain(model_parts)
            .filter(|uv| uv.name.contains("Layer") || entry.render_layers)
            .collect()
    }

    pub(crate) fn get_overlay(&self, uv: &UvImage) -> Option<&UvImage> {
        self.model_overlays
            .iter()
            .find(|other| other.name.eq(&uv.name))
    }
}
