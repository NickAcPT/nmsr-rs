#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]

#![allow(missing_docs)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::redundant_pub_crate)]
#![allow(clippy::unused_async)]

pub mod model;
mod routes;
mod utils;

use crate::routes::render;
use crate::routes::NMSRState;
use crate::utils::tracing::NmsrTracing;

use anyhow::Context;
use axum::routing::post;
use opentelemetry::StringValue;
use tower_http::cors::AllowMethods;
use tower_http::cors::Any;
use tower_http::cors::CorsLayer;
use tracing::info_span;
use tracing_subscriber::EnvFilter;
use twelf::Layer;
use utils::config::TracingConfiguration;
pub use utils::{caching, config, error};

use crate::utils::config::NmsrConfiguration;

use std::{net::SocketAddr, path::PathBuf};

use axum::{routing::get, Router, ServiceExt};
use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace, Resource},
    KeyValue,
};
use opentelemetry_otlp::{new_exporter, WithExportConfig};
use tokio::{main, signal};
use tower::ServiceBuilder;
use tower_http::{request_id::MakeRequestUuid, ServiceBuilderExt};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[main]
async fn main() -> anyhow::Result<()> {
    let init_guard = info_span!("NMSRaaS init").entered();
    let toml_path: PathBuf = "config.toml".into();
    let toml_layer = Some(Layer::Toml(toml_path.clone())).filter(|_| toml_path.exists());

    let layers: Vec<_> = vec![
        Some(Layer::DefaultTrait),
        toml_layer,
        Some(Layer::Env(Some("NMSR_".into()))),
    ]
    .into_iter()
    .flatten()
    .collect();

    let config = NmsrConfiguration::with_layers(&layers).context("Unable to load configuration")?;

    setup_tracing(config.tracing.as_ref())?;

    info!("Loaded configuration: {:#?}", config);

    let state = NMSRState::new(&config).await?;

    state.init().await?;

    let adapter = &state.graphics_context.adapter.get_info();
    let samples = &state.graphics_context.multisampling_strategy;

    info!(
        "Initialized state with adapter {:?} and using {:?} multisampling strategy",
        adapter, samples
    );

    // build our application with a route
    let router = Router::new()
        .route("/", get(root))
        .route("/:mode/:texture", get(render))
        .route("/:mode", post(render))
        .with_state(state);

    let trace_layer = NmsrTracing::new_trace_layer();

    let app = ServiceBuilder::new()
        .set_x_request_id(MakeRequestUuid)
        .layer(trace_layer)
        .propagate_x_request_id()
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(AllowMethods::any()),
        )
        .service(router);

    let addr = (config.server.address + ":" + &config.server.port.to_string()).parse()?;

    tracing::info!("Listening on {}", &addr);

    drop(init_guard);

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn setup_tracing(tracing: Option<&TracingConfiguration>) -> anyhow::Result<()> {
    let base_filter = "info,h2=off,wgpu_core=warn,wgpu_hal=error,naga=warn";
    let otel_filter = format!("{base_filter},nmsr_aas=trace,nmsr_rendering=trace");

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| base_filter.into());
    let otel_env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| otel_filter.into());

    let fmt_layer = tracing_subscriber::Layer::with_filter(
        tracing_subscriber::fmt::layer()
            .compact()
            .with_line_number(false)
            .with_file(false),
        env_filter,
    );

    global::set_text_map_propagator(TraceContextPropagator::new());

    let registry = tracing_subscriber::registry().with(fmt_layer);

    if let Some(tracing) = tracing {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(new_exporter().tonic().with_endpoint(&tracing.endpoint))
            .with_trace_config(
                trace::config().with_resource(Resource::new(vec![KeyValue::new(
                    "service.name",
                    Into::<StringValue>::into(tracing.service_name.clone()),
                )])),
            )
            .install_batch(opentelemetry::runtime::Tokio)?;

        let otel_layer = tracing_subscriber::Layer::with_filter(
            tracing_opentelemetry::layer().with_tracer(tracer),
            otel_env_filter,
        );

        registry.with(otel_layer).init();
    } else {
        registry.init();
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Received shutdown signal... Shutting down.");

    global::shutdown_tracer_provider();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}
