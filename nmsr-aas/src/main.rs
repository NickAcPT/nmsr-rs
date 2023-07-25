use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;
use std::time::Duration;

use actix_cors::Cors;
#[cfg(not(feature = "tracing"))]
use actix_web::middleware::Logger;
use actix_web::rt::time;
use actix_web::{web::Data, App, HttpServer};
use clap::Parser;
use parking_lot::RwLock;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use tracing::{debug, info, info_span};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt, EnvFilter, Layer};
use tracing_subscriber::{layer::SubscriberExt, Registry};

use routes::{
    get_skin_route::get_skin, get_skin_route::get_skin_head, index_route::index,
    render_body_route::render, render_body_route::render_head,
};
#[cfg(feature = "tracing")]
use {
    opentelemetry::{
        global,
        sdk::{propagation::TraceContextPropagator, trace, Resource},
        KeyValue,
    },
    opentelemetry_otlp::WithExportConfig,
    tracing_actix_web::TracingLogger,
    crate::utils::tracing_span::NMSRRootSpanBuilder,
};

use crate::config::{MojankConfiguration, ServerConfiguration};
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
    let mojank_config = Data::new(config_ref.mojank.clone());

    // Setup the tracing here
    // What we want to do is basically, if we compile with tracing feature, use opentelemetry tracing
    // Otherwise, use the default tracing which outputs to stdout
    setup_tracing_config(&config)?;

    let server_init_span = info_span!("NMSRaaS init", config = ?config.clone()).entered();

    info!("Starting NMSRaaS - NickAc's Minecraft Skin Renderer as a Service");

    info!("Loading parts manager...");
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

            let _span = info_span!("clean_cache").entered();

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

    let mojang_requests_client = build_mojang_request_client(&mojank_config)?;

    info!("Starting server...");

    let server = HttpServer::new(move || {
        let cors = Cors::default().allow_any_origin().allow_any_header();

        #[cfg(not(feature = "tracing"))]
        let logger = Logger::default();
        #[cfg(feature = "tracing")]
        let logger = TracingLogger::<NMSRRootSpanBuilder>::new();

        let app = App::new();

        #[cfg(feature = "tracing")]
        let app = app.wrap(utils::tracing_headers::TraceIdHeader);

        app.wrap(logger)
            .wrap(cors)
            .app_data(Data::new(manager.clone()))
            .app_data(Data::new(mojang_requests_client.clone()))
            .app_data(mojank_config.clone())
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

    drop(server_init_span);

    server.run().await?;

    // Ensure all spans have been shipped.
    #[cfg(feature = "tracing")]
    {
        global::shutdown_tracer_provider();
    }

    Ok(())
}

fn build_mojang_request_client(mojank_config: &Data<MojankConfiguration>) -> Result<ClientWithMiddleware> {
    let mut mojang_requests_client = reqwest::Client::builder()
        .user_agent(format!("NMSR as a Service/{}", env!("CARGO_PKG_VERSION")));

    if let Some(experimental_http2_prior_knowledge) = mojank_config.experimental_http2_prior_knowledge {
        if experimental_http2_prior_knowledge { mojang_requests_client = mojang_requests_client.http2_prior_knowledge(); }
    }

    if let Some(experimental_http2_keep_alive_while_idle) = mojank_config.experimental_http2_keep_alive_while_idle {
        mojang_requests_client = mojang_requests_client.http2_keep_alive_while_idle(experimental_http2_keep_alive_while_idle);
    }

    if let Some(experimental_http2_keep_alive_interval) = mojank_config.experimental_http2_keep_alive_interval {
        mojang_requests_client = mojang_requests_client.http2_keep_alive_interval(Duration::from_secs(experimental_http2_keep_alive_interval));
    }

    if let Some(experimental_http2_keep_alive_timeout) = mojank_config.experimental_http2_keep_alive_timeout {
        mojang_requests_client = mojang_requests_client.http2_keep_alive_timeout(Duration::from_secs(experimental_http2_keep_alive_timeout));
    }

    Ok(ClientBuilder::new(mojang_requests_client.build()?)
        .with(TracingMiddleware::default())
        .build())
}

fn setup_tracing_config(config: &Data<ServerConfiguration>) -> Result<()> {
    #[cfg(feature = "tracing")]
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Here, we create a filter that will only debug output messages from our crates and errors from actix
    let fmt_filter = EnvFilter::from_str("none,nmsr_aas=info,tracing_actix_web=error")
        .expect("Failed to create env filter for fmt");

    // Layer for pretty printing and exporting to stdout
    let fmt_layer = fmt::layer().pretty().with_filter(fmt_filter);

    // Use the tracing subscriber `Registry`, or any other subscriber
    // that impls `LookupSpan`
    let subscriber = Registry::default()
        // Add layer for pretty printing and exporting to stdout
        .with(fmt_layer);

    // Add the tracing layer to the subscriber
    #[cfg(feature = "tracing")]
    let subscriber = subscriber.with({
        // Create a new OpenTelemetry pipeline
        let otel_tracer =
            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(&config.tracing.otel_endpoint),
                )
                .with_trace_config(trace::config().with_resource(Resource::new(vec![
                    KeyValue::new("service.name", config.tracing.otel_service_name.clone()),
                ])))
                .install_batch(opentelemetry::runtime::TokioCurrentThread)?;

        // Here we create a filter that will let through our crates' messages and the ones from actix_web
        let otel_filter = EnvFilter::from_str(
            "none,nmsr_aas=trace,nmsr_lib=trace,tracing_actix_web=trace,reqwest_tracing=debug,rustls=debug",
        )
        .expect("Failed to create env filter for otel");

        // Create a tracing layer
        tracing_opentelemetry::layer()
            .with_tracer(otel_tracer)
            .with_filter(otel_filter)
    });

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    LogTracer::builder()
        .ignore_crate("hyper")
        .ignore_crate("h2")
        .init()?;

    Ok(())
}
