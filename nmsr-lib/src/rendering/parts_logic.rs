use either::Either;

use crate::parts::manager::PartsManager;
#[cfg(feature = "ears")]
use crate::rendering::ears_parts_logic::get_ears_parts;
use crate::rendering::entry::RenderingEntry;
use crate::uv::uv_magic::UvImage;

impl PartsManager {
    pub(crate) fn get_parts(&self, entry: &RenderingEntry) -> Vec<&UvImage> {
        let required_parts = self.all_parts.iter();

        let model_parts = self
            .model_parts
            .iter()
            .filter(|uv| uv.name.starts_with(entry.model.get_dir_name()));

        let iterator = required_parts
            .chain(model_parts)
            .filter(|uv| !uv.name.contains("Layer") || entry.render_layers);

        #[cfg(feature = "ears")]
        {
            let mut existing_parts: Vec<_> = iterator.collect();

            if let Some(ears_manager) = &self.ears_parts_manager {
                if let Some(features) = &entry.ears_features {
                    let ears_parts = get_ears_parts(features, &entry.model).into_iter().map(|p| {
                        ears_manager
                            .all_parts
                            .iter()
                            .chain(
                                ears_manager
                                    .model_parts
                                    .iter()
                                    .filter(|uv| uv.name.starts_with(entry.model.get_dir_name())),
                            )
                            .find(|uv| {
                                uv.name == p
                            })
                    });

                    for part in ears_parts.flatten() {
                        existing_parts.push(part);
                    }
                }
            }

            existing_parts
        }
        #[cfg(not(feature = "ears"))]
        {
            iterator.collect()
        }
    }

    pub(crate) fn get_overlay(&self, uv: &UvImage) -> Option<&UvImage> {
        let mut iterator = Either::Left(self.model_overlays.iter());

        #[cfg(feature = "ears")]
        {
            if let Some(ears_manager) = &self.ears_parts_manager {
                iterator = Either::Right(
                    iterator
                        .unwrap_left()
                        .chain(ears_manager.model_overlays.iter()),
                );
            }
        }
        #[cfg(not(feature = "ears"))]{
            iterator = Either::Right(iterator.unwrap_left());
        }

        iterator.find(|other| other.name.eq(&uv.name))
    }
}
