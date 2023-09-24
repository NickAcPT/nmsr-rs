use std::ops::Deref;

use nmsr_rendering_blockbench_model_generator_experiment::{
    blockbench,
    generator::{DefaultImageIO, new_model_generator_without_part_context},
    nmsr_rendering::high_level::{model::PlayerModel, types::PlayerPartTextureType},
};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue, UnwrapThrowExt};

extern crate alloc;

#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

// SAFETY: This application is single threaded, so using AssumeSingleThreaded is allowed.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
    unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

#[wasm_bindgen]
pub struct ConversionResult {
    value: JsValue,
    is_error: bool,
}

#[wasm_bindgen]
impl ConversionResult {
    pub fn value(&self) -> JsValue {
        self.value.clone()
    }

    pub fn is_error(&self) -> bool {
        self.is_error
    }
}

#[wasm_bindgen]
pub enum WasmPlayerModel {
    Steve,
    Alex,
}

impl Deref for WasmPlayerModel {
    type Target = PlayerModel;

    fn deref(&self) -> &Self::Target {
        match self {
            WasmPlayerModel::Steve => &PlayerModel::Steve,
            WasmPlayerModel::Alex => &PlayerModel::Alex,
        }
    }
}

#[wasm_bindgen]
pub fn generate_blockbench_model(
    skin_bytes: &[u8],
    model: WasmPlayerModel,
    layers: bool,
) -> ConversionResult {
    console_error_panic_hook::set_once();

    let mut project = new_model_generator_without_part_context(*model, layers, DefaultImageIO);

    project.load_texture(PlayerPartTextureType::Skin, &skin_bytes).unwrap_throw();

    let result = blockbench::generate_project(project);

    match result {
        Ok(result) => ConversionResult {
            value: result,
            is_error: false,
        },
        Err(e) => ConversionResult {
            value: e.into(),
            is_error: true,
        },
    }
}
