use std::{error::Error, ops::Deref};

use nmsr_rendering_blockbench_model_generator_experiment::{
    blockbench,
    generator::ModelGenerationProject,
    nmsr_rendering::high_level::{model::PlayerModel, types::PlayerPartTextureType},
};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct ConversionResult {
    result: Option<String>,
    error: Option<String>,
}

#[wasm_bindgen]
impl ConversionResult {
    pub fn result(&self) -> Option<String> {
        self.result.clone()
    }

    pub fn error(&self) -> Option<String> {
        self.error.clone()
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

    let mut project = ModelGenerationProject::new(*model, layers);

    let texture_result = project.load_texture(PlayerPartTextureType::Skin, &skin_bytes);

    let result = texture_result.and_then(|_| blockbench::generate_project(project));

    match result {
        Ok(result) => ConversionResult {
            result: Some(result),
            error: None,
        },
        Err(error) => ConversionResult {
            result: None,
            error: Some(error.to_string()),
        },
    }
}
