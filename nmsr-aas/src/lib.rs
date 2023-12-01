#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::redundant_pub_crate,
    clippy::unused_async,
    clippy::diverging_sub_expression,
    clippy::future_not_send
)]

pub mod model;
mod routes;
mod utils;

use std::collections::HashMap;

// Make function to log
fn log<T: Into<JsValue>>(_s: T) {
    //web_sys::console::log_1(&s.into());
}

use crate::{
    model::request::{cache::CacheBias, entry::RenderRequestEntry},
    routes::{render, render_get_warning, render_post_warning, NMSRState, query::RenderRequestMultipartParams},
    utils::config::{
        ModelCacheConfiguration, MojankConfiguration, NmsrConfiguration, RenderingConfiguration,
        ServerConfiguration,
    },
};
use axum::{
    body::Body,
    response::Response,
    routing::{get, post},
    Router,
};
use chrono::Duration;
use http::{response::Parts, HeaderName, HeaderValue, Method, Request, Uri};
use http_body_util::BodyExt;
use tower::ServiceExt;

pub use utils::{caching, config, error};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::{Reflect, Uint8Array};

type Result<T> = std::result::Result<T, JsError>;

#[wasm_bindgen]
pub struct WasmNMSRState(Router);

#[wasm_bindgen]
pub async fn init_nmsr_aas() -> Result<WasmNMSRState> {
    console_error_panic_hook::set_once();

    let cache_biases = HashMap::new();

    let config = NmsrConfiguration {
        server: ServerConfiguration {
            address: "0.0.0.0".to_string(),
            port: 8621,
            static_files_directory: None,
        },
        tracing: None,
        caching: ModelCacheConfiguration {
            cleanup_interval: std::time::Duration::ZERO,
            resolve_cache_duration: std::time::Duration::from_secs(60 * 15),
            texture_cache_duration: std::time::Duration::from_secs(60 * 60 * 48),
            cache_biases,
        },
        mojank: MojankConfiguration {
            session_server: "https://corsjangsessionserver.b-cdn.net".to_string(),
            textures_server: "https://textures.minecraft.net".to_string(),
            geysermc_api_server: "https://api.geysermc.org/".to_string(),
            session_server_rate_limit: 10,
        },
        rendering: Some(RenderingConfiguration {
            sample_count: 1,
            use_smaa: false,
        }),
        features: None,
    };

    let state = NMSRState::new(&config).await?;

    state.init().await?;

    let router = Router::new()
        .route("/:mode/:texture", get(render))
        .route("/:mode/:texture", post(render_post_warning))
        .route("/:mode", get(render_get_warning))
        .route("/:mode", post(render))
        .with_state(state);

    Ok(WasmNMSRState(router))
}

#[wasm_bindgen]
pub struct WasmRequest {
    method: Method,
    uri: Uri,
    headers: JsValue,
    body: Option<Vec<u8>>,
    form_data: Option<JsValue>,
}

#[wasm_bindgen]
impl WasmRequest {
    pub fn new_non_post(
        method: String,
        uri: String,
        headers: JsValue,
        body: Vec<u8>,
    ) -> Result<WasmRequest> {
        Ok(Self {
            method: method.parse()?,
            uri: uri.parse()?,
            headers,
            body: Some(body),
            form_data: None,
        })
    }
    pub fn new_post(
        method: String,
        uri: String,
        headers: JsValue,
        form_data: JsValue,
    ) -> Result<WasmRequest> {
        Ok(Self {
            method: method.parse()?,
            uri: uri.parse()?,
            headers,
            body: None,
            form_data: Some(form_data),
        })
    }

    fn convert_headers(&self) -> Result<http::HeaderMap> {
        let mut header_map = http::HeaderMap::new();

        // Convert the headers from a JS object to a HashMap
        let headers: HashMap<String, String> = Reflect::own_keys(&self.headers)
            .expect_throw("Failed to get own keys")
            .iter()
            .map(|key| {
                let value = Reflect::get(&self.headers, &key)
                    .unwrap()
                    .as_string()
                    .unwrap();
                (key.as_string().unwrap(), value)
            })
            .collect();

        header_map.extend(headers.into_iter().map(|(key, value)| {
            let key: HeaderName = key.parse().unwrap();
            let value: HeaderValue = value.parse().unwrap();
            (key, value)
        }));

        Ok(header_map)
    }

    fn to_request(self) -> Result<Request<axum::body::Body>> {
        let headers = self.convert_headers()?;
        let body = self.body.unwrap_or(vec![]).into();

        let mut builder = Request::builder().method(self.method).uri(self.uri);

        if let Some(header_map) = builder.headers_mut() {
            header_map.extend(headers.into_iter());
        }
        
        if let Some(form_data) = self.form_data {
            let form_data: RenderRequestMultipartParams = serde_wasm_bindgen::from_value::<RenderRequestMultipartParams>(form_data)
            .map_err(|e| JsError::new(&format!("Failed to decode multipart form data: {}", e)))?;
        
            builder = builder.extension(form_data);
        }

        Ok(builder.body(body)?)
    }
}

#[wasm_bindgen]
pub struct NmsrWasmResponse {
    parts: Parts,
    body: Body,
}

#[wasm_bindgen]
impl NmsrWasmResponse {
    pub fn get_status(&self) -> u16 {
        self.parts.status.as_u16()
    }

    pub async fn get_body(self, headers: JsValue) -> Result<Uint8Array> {
        for (name, value) in self.parts.headers.iter() {
            Reflect::set(
                &headers,
                &name.as_str().into(),
                &value.to_str().unwrap().into(),
            )
            .unwrap();
        }

        let response_bytes = self.body.collect().await?.to_bytes().to_vec();
        crate::log(format!("Response bytes: {:?}", response_bytes.len()));

        crate::log("Creating response array");
        let response_array = Uint8Array::from(response_bytes.as_slice());

        Ok(response_array)
    }
}

#[wasm_bindgen]
pub async fn handle_request(request: WasmRequest) -> Result<NmsrWasmResponse> {
    let state = init_nmsr_aas().await?;

    let request = request.to_request()?;

    crate::log("Handling request");

    let response = state.0.oneshot(request).await?;

    crate::log(format!("Response: {:?}", response));

    let (parts, response) = response.into_parts();

    Ok(NmsrWasmResponse {
        parts,
        body: response,
    })
}
