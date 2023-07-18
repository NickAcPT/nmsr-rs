use actix_cors::Cors;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

use actix_web::rt::time;
use actix_web::{web::Data, App, HttpServer};
use clap::Parser;
use parking_lot::RwLock;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use tracing::{debug, info};

#[cfg(not(feature = "tracing"))]
use actix_web::middleware::Logger;
use tracing::level_filters::LevelFilter;
#[cfg(feature = "tracing")]
use {
    opentelemetry::{
        sdk::{trace, trace::Tracer, Resource},
        KeyValue,
    },
    opentelemetry_otlp::WithExportConfig,
    tracing_actix_web::TracingLogger,
    tracing_opentelemetry::OpenTelemetryLayer,
};

use tracing_subscriber::{
    fmt::format::{Format, Pretty},
    fmt::Subscriber,
    layer::{Layered, SubscriberExt},
    FmtSubscriber,
    Registry
};

use routes::{
    get_skin_route::get_skin, get_skin_route::get_skin_head, index_route::index,
    render_body_route::render, render_body_route::render_head,
};

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

    //env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));m

    // Setup the tracing here
    // What we want to do is basically, if we compile with tracing feature, use opentelemetry tracing
    // Otherwise, use the default tracing which outputs to stdout
    setup_tracing_config(&config)?;

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
                cache_manager
                    .write()
                    .purge_expired_uuid_to_skin_hash_cache();
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

        #[cfg(not(feature = "tracing"))]
        let logger = Logger::default();
        #[cfg(feature = "tracing")]
        let logger = TracingLogger::default();

        App::new()
            .wrap(logger)
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

    info!("Binding to {:?}...", &addr);

    let server = if let Some(config) = tls_config {
        server.bind_rustls(addr, config)?
    } else {
        server.bind(addr)?
    };

    server.run().await?;
    Ok(())
}

fn setup_tracing_config(config: &Data<ServerConfiguration>) -> Result<()> {
    let subscriber = get_tracing_subscriber(config)?;

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    Ok(())
}

#[cfg(not(feature = "tracing"))]
fn get_tracing_subscriber(
    _config: &Data<ServerConfiguration>,
) -> Result<Subscriber<Pretty, Format<Pretty>>> {
    Ok(FmtSubscriber::builder()
        .pretty()
        .with_max_level(LevelFilter::DEBUG)
        .finish())
}

#[cfg(feature = "tracing")]
fn get_tracing_subscriber(
    config: &Data<ServerConfiguration>,
) -> Result<Layered<OpenTelemetryLayer<Registry, Tracer>, Registry, Registry>> {
    // Create a new OpenTelemetry pipeline
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.tracing.otel_endpoint),
        )
        .with_trace_config(
            trace::config().with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                (&config.tracing.otel_service_name).clone(),
            )])),
        )
        .install_simple()?;

    // Create a tracing layer with the configured tracer
    let layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Use the tracing subscriber `Registry`, or any other subscriber
    // that impls `LookupSpan`
    let subscriber = Registry::default().with(layer);

    Ok(subscriber)
}
