mod get_skin;

pub use get_skin::get_skin;

use std::sync::Arc;

use crate::{
    config::NmsrConfiguration,
    error::Result,
    model::{
        request::cache::ModelCache,
        resolver::{mojang::client::MojangClient, RenderRequestResolver},
    },
};

#[derive(Clone)]
pub struct NMSRState {
    resolver: Arc<RenderRequestResolver>,
}

impl NMSRState {
    pub fn new(config: &NmsrConfiguration) -> Result<Self> {
        let mojang_client = MojangClient::new(Arc::new(config.mojank.clone()))?;
        let model_cache = ModelCache::new("cache".into(), config.caching.clone())?;

        let resolver = RenderRequestResolver::new(model_cache, Arc::new(mojang_client));

        Ok(Self {
            resolver: Arc::new(resolver),
        })
    }
}
