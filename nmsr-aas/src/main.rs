mod mojang;
mod routes;
mod utils;

use crate::mojang::caching::MojangCacheManager;
use crate::utils::Result;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use log::{debug, info};
use nmsr_lib::parts::manager::PartsManager;
use routes::{
    get_skin_route::get_skin, index_route::index, render_full_body_route::render_full_body,
};

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    info!("Starting NMSRaaS - NickAc's Minecraft Skin Renderer as a Service");

    debug!("Loading parts manager...");
    let start = std::time::Instant::now();
    let parts_manager = PartsManager::new("parts")?;
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
            .app_data(Data::new(parts_manager.clone()))
            .app_data(Data::new(mojang_requests_client.clone()))
            .app_data(Data::new(cache_manager.clone()))
            .service(index)
            .service(render_full_body)
            .service(get_skin)
    });

    let server = server.bind(("0.0.0.0", 8080))?;

    info!("Server started on port 8080 (http://localhost:8080)");

    server.run().await?;
    Ok(())
}
