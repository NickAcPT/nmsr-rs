use std::sync::Arc;

use image::RgbaImage;
use nmsr_rendering::{
    errors::NMSRRenderingError,
    high_level::{
        model::ArmorMaterial,
        parts::{
            part::Part,
            provider::{PartsProvider, PlayerPartProviderContext, PlayerPartsProvider},
        },
        types::{PlayerBodyPartType, PlayerPartTextureType},
        utils::parts::primitive_convert,
    },
    low_level::primitives::mesh::{Mesh, PrimitiveDispatch},
};

use nmsr_rasterizer_test::{
    camera::Camera,
    model::{RenderEntry, Size},
    shader::{ShaderState, SunInformation},
};
use tracing::instrument;

pub struct Scene<'a, M: ArmorMaterial> {
    camera: Camera,
    lighting: SunInformation,
    size: Size,
    entry: RenderEntry,
    pub(crate) parts_context: &'a PlayerPartProviderContext<M>,
    parts: Vec<PlayerBodyPartType>,
    shader_states: Vec<ShaderState>,
}

impl<'a, M: ArmorMaterial + derive_more::Debug> derive_more::Debug for Scene<'a, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scene").field("camera", &self.camera).field("lighting", &self.lighting).field("size", &self.size).field("entry", &self.entry).field("parts_context", &self.parts_context).field("parts", &self.parts).field("shader_states", &self.shader_states).finish()
    }
}

impl<'a, M: ArmorMaterial + derive_more::Debug> Scene<'a, M> {
    pub fn new(
        mut camera: Camera,
        lighting: SunInformation,
        size: Size,
        parts_context: &'a PlayerPartProviderContext<M>,
        parts: &[PlayerBodyPartType],
    ) -> Self {
        if let None = camera.get_size() {
            camera.set_size(Some(size));
        }

        Self {
            camera,
            lighting,
            size,
            entry: RenderEntry::new(camera.get_size().unwrap()),
            parts_context,
            parts: parts.to_vec(),
            shader_states: Vec::new(),
        }
    }

    
    #[instrument(skip(self, image))]
    pub fn set_texture(&mut self, texture: PlayerPartTextureType, image: Arc<RgbaImage>) {
        let providers = [
            PlayerPartsProvider::Minecraft,
            #[cfg(feature = "ears")]
            PlayerPartsProvider::Ears,
        ];

        let parts = providers
            .iter()
            .flat_map(|provider| {
                self.parts
                    .iter()
                    .flat_map(|part| provider.get_parts(&self.parts_context, *part))
            })
            .filter(|p| p.get_texture() == texture)
            .collect::<Vec<Part>>();

        let parts = parts
            .into_iter()
            .map(|p| primitive_convert(&p))
            .collect::<Vec<_>>();

        self.shader_states.push(ShaderState::new_with_primitive(
            self.camera,
            image,
            self.lighting,
            PrimitiveDispatch::Mesh(Mesh::new(parts)),
        ));
    }

    #[instrument(skip_all)]
    pub fn render(&mut self) -> Result<(), NMSRRenderingError> {
        for state in &mut self.shader_states {
            state.update();
            self.entry.draw(state);
        }

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn copy_output_texture(&self) -> &[u8] {
        let size = self.size;
        &self
            .entry
            .textures
            .output
            .get(0..((size.width * size.height * 4) as usize))
            .expect("Failed to copy output texture")
    }
}
