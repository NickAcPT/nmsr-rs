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

use crate::{
    model::request::{cache::CacheBias, entry::RenderRequestEntry},
    routes::{render, render_get_warning, render_post_warning, NMSRState},
    utils::config::{
        ModelCacheConfiguration, MojankConfiguration, NmsrConfiguration, RenderingConfiguration,
        ServerConfiguration,
    },
};
use axum::{
    routing::{get, post},
    Router,
};
use chrono::Duration;
use http::{Method, Request, Uri, HeaderValue, HeaderName};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::uuid;

pub use utils::{caching, config, error};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::{Reflect, Uint8Array};

type Result<T> = std::result::Result<T, JsError>;

#[wasm_bindgen]
pub struct WasmNMSRState(Router);

#[wasm_bindgen]
pub async fn init_nmsr_aas() -> Result<WasmNMSRState> {
    let cache_biases = {
        let mut map = HashMap::new();
        map.insert(
            RenderRequestEntry::TextureHash(
                "e47682cc9c509546d22ae532fb3bd71f1e53b0374432694bf794e070cbcd1b0b".to_string(),
            ),
            CacheBias::CacheIndefinitely,
        );
        map.insert(
            RenderRequestEntry::TextureHash(
                "1c7e0389ed5d81acb56a5e79b817426e628114a270b097bfab3a7ebf97f3b1fb".to_string(),
            ),
            CacheBias::CacheIndefinitely,
        );
        map.insert(
            RenderRequestEntry::MojangPlayerUuid(uuid!("ad4569f3-7576-4376-a7c7-8e8cfcd9b832")),
            CacheBias::KeepCachedFor(Duration::hours(6).to_std().unwrap()),
        );
        map
    };

    let config = NmsrConfiguration {
        server: ServerConfiguration {
            address: "0.0.0.0".to_string(),
            port: 8621,
            static_files_directory: None,
        },
        tracing: None,
        caching: ModelCacheConfiguration {
            cleanup_interval: std::time::Duration::ZERO,
            resolve_cache_duration: std::time::Duration::ZERO,
            texture_cache_duration: std::time::Duration::ZERO,
            cache_biases,
        },
        mojank: MojankConfiguration {
            session_server: "https://sessionserver.mojang.com".to_string(),
            textures_server: "https://textures.minecraft.net".to_string(),
            geysermc_api_server: "https://api.geysermc.org/".to_string(),
            session_server_rate_limit: 10,
        },
        rendering: Some(RenderingConfiguration {
            sample_count: 1,
            use_smaa: true,
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
    body: Vec<u8>,
}

#[wasm_bindgen]
impl WasmRequest {
    pub fn new(method: String, uri: String, headers: JsValue, body: Vec<u8>) -> Result<WasmRequest> {
        Ok(
            Self {
                method: method.parse()?,
                uri: uri.parse()?,
                headers,
                body,
            }
        )
    }
    
    fn convert_headers(&self) -> Result<http::HeaderMap> {
        let mut header_map = http::HeaderMap::new();
        
        // Convert the headers from a JS object to a HashMap
        let headers: HashMap<String, String> = Reflect::own_keys(&self.headers).expect_throw("Failed to get own keys")
            .iter()
            .map(|key| {
                let value = Reflect::get(&self.headers, &key).unwrap().as_string().unwrap();
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
        let body = self.body.into();
        
        let mut builder = Request::builder()
            .method(self.method)
            .uri(self.uri);
        
        if let Some(header_map) = builder.headers_mut() {
            header_map.extend(headers.into_iter());
        }
        
        Ok(builder.body(body)?)
    }
}

#[wasm_bindgen]
pub async fn handle_request(request: WasmRequest) -> Result<Uint8Array> {
    let state = init_nmsr_aas().await?;

    let request = request.to_request()?;
    
    let response = state.0
        .oneshot(request)
        .await?;
    
    let response = response.into_body();
    
    let response_bytes = response.collect().await?.to_bytes().to_vec();
    
    Ok(Uint8Array::from(&response_bytes[..]))
}