mod manager;
mod mojang;
mod routes;
mod utils;

use crate::manager::NMSRaaSManager;
use crate::mojang::caching::MojangCacheManager;
use crate::utils::Result;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use log::{debug, info};
use parking_lot::RwLock;
use routes::{get_skin_route::get_skin, index_route::index, render_body_route::render};

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    info!("Starting NMSRaaS - NickAc's Minecraft Skin Renderer as a Service");

    debug!("Loading parts manager...");
    let start = std::time::Instant::now();
    let manager = NMSRaaSManager::new("parts")?;
    info!("Parts manager loaded in {}ms", start.elapsed().as_millis());

    let cache_manager = MojangCacheManager::init("cache")?;
    cache_manager.cleanup_old_files()?;

    let mojang_requests_client = reqwest::Client::builder()
        .user_agent(format!("NMSR as a Service/{}", env!("CARGO_PKG_VERSION")))
        .build()?;

    info!("Starting server...");

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(Data::new(manager.clone()))
            .app_data(Data::new(mojang_requests_client.clone()))
            .app_data(Data::new(RwLock::new(cache_manager.clone())))
            .service(index)
            .service(get_skin)
            .service(render)
    });

    let server = server.bind(("0.0.0.0", 8080))?;

    info!("Server started on port 8080 (http://localhost:8080)");

    server.run().await?;
    Ok(())
}
