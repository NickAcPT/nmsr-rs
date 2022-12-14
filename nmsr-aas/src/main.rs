use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;
use actix_cors::Cors;

use actix_web::rt::time;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use clap::Parser;
use log::{debug, info};
use parking_lot::RwLock;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};

use routes::{get_skin_route::get_skin, get_skin_route::get_skin_head, index_route::index, render_body_route::render, render_body_route::render_head};

use crate::config::ServerConfiguration;
use crate::manager::NMSRaaSManager;
use crate::mojang::caching::MojangCacheManager;
use crate::routes::index_route::index_head;
use crate::utils::Result;

mod config;
mod manager;
mod mojang;
mod routes;
mod utils;

#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    config: Option<String>,
}

#[actix_web::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config = match args.config {
        Some(path) => toml::from_str(fs::read_to_string(path)?.as_str())?,
        None => ServerConfiguration::default(),
    };
    let config = Data::new(config);
    let config_ref = config.clone().into_inner();
    let cache_config = Data::new(config_ref.cache.clone());

    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    info!("Starting NMSRaaS - NickAc's Minecraft Skin Renderer as a Service");

    debug!("Loading parts manager...");
    let start = std::time::Instant::now();
    let manager = NMSRaaSManager::new(&config.parts)?;
    info!("Parts manager loaded in {}ms", start.elapsed().as_millis());

    let cache_manager = MojangCacheManager::init(
        "cache",
        config.cache.image_cache_expiry,
        config.cache.mojang_profile_request_expiry,
        config.cache.mojang_profile_requests_per_second,
    )?;

    let cache_manager = Data::new(RwLock::new(cache_manager));
    let cache_ref = cache_manager.clone().into_inner();

    actix_web::rt::spawn(async move {
        let config = config_ref;
        let cache_manager = cache_ref;
        let mut interval =
            time::interval(Duration::from_secs(config.cache.cleanup_interval as u64));
        loop {
            interval.tick().await;

            debug!("Cleaning up cache...");
            cache_manager
                .read()
                .cleanup_old_files()
                .expect("Failed to cleanup cache");

            {
                cache_manager.write().purge_expired_uuid_to_skin_hash_cache();
            }
            debug!("Cache cleaned up");
        }
    });

    let mojang_requests_client = reqwest::Client::builder()
        .user_agent(format!("NMSR as a Service/{}", env!("CARGO_PKG_VERSION")))
        .build()?;

    info!("Starting server...");

    let server = HttpServer::new(move || {
        let cors = Cors::default().allow_any_origin().allow_any_header();
        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(Data::new(manager.clone()))
            .app_data(Data::new(mojang_requests_client.clone()))
            .app_data(cache_manager.clone())
            .app_data(cache_config.clone())
            .service(index)
            .service(index_head)
            .service(get_skin)
            .service(get_skin_head)
            .service(render)
            .service(render_head)
    });

    let tls_config = if let Some(tls) = &config.tls {
        let private_key = &tls.private_key;

        let certificate_chain = &tls.certificate_chain;

        let cert_file = &mut BufReader::new(File::open(certificate_chain)?);
        let key_file = &mut BufReader::new(File::open(private_key)?);

        let cert_chain = certs(cert_file)
            .unwrap()
            .into_iter()
            .map(Certificate)
            .collect();
        let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
            .unwrap()
            .into_iter()
            .map(PrivateKey)
            .collect();

        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, keys.remove(0))?;

        Some(config)
    } else {
        None
    };

    let addr = (config.address.clone(), config.port);

    let server = if let Some(config) = tls_config {
        server.bind_rustls(addr, config)?
    } else {
        server.bind(addr)?
    };

    server.run().await?;
    Ok(())
}
