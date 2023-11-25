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

use crate::{
    routes::{render, render_get_warning, render_post_warning, NMSRState},
    utils::tracing::NmsrTracing,
};

use crate::utils::config::NmsrConfiguration;
use anyhow::Context;
use axum::routing::post;
use axum::{routing::get, Router};
use http::HeaderName;
use opentelemetry::StringValue;
use opentelemetry::{
    global,
    KeyValue,
};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace, Resource};
use opentelemetry_otlp::{new_exporter, WithExportConfig};
use std::{net::SocketAddr, path::PathBuf};
use tokio::{main, signal};
use tower_http::{
    cors::{AllowMethods, Any, CorsLayer},
    request_id::{PropagateRequestIdLayer, SetRequestIdLayer},
    services::ServeDir, normalize_path::NormalizePathLayer,
};
use tower_http::request_id::MakeRequestUuid;
use tracing::info;
use tracing::info_span;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use twelf::Layer;
use utils::config::TracingConfiguration;
pub use utils::{caching, config, error};

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
        .route("/:mode/:texture", get(render))
        .route("/:mode/:texture", post(render_post_warning))
        .route("/:mode", get(render_get_warning))
        .route("/:mode", post(render))
        .with_state(state);

    let router = if let Some(path) = config.server.static_files_directory {
        let serve_dir = ServeDir::new(path)
            .precompressed_br()
            .precompressed_gzip()
            .call_fallback_on_method_not_allowed(true)
            .fallback(router.layer(NormalizePathLayer::trim_trailing_slash()));

        Router::new().nest_service("/", serve_dir)
    } else {
        router.route("/", get(root))
    };

    let trace_layer: tower_http::trace::TraceLayer<
        tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
        NmsrTracing<axum::body::Body>,
        NmsrTracing<axum::body::Body>,
        NmsrTracing<axum::body::Body>,
        tower_http::trace::DefaultOnBodyChunk,
        (),
        NmsrTracing<axum::body::Body>,
    > = NmsrTracing::new_trace_layer();

    let app = router
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ))
        .layer(trace_layer)
        .layer(PropagateRequestIdLayer::new(HeaderName::from_static(
            "x-request-id",
        )))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(AllowMethods::any()),
        );

    let addr: SocketAddr =
        (config.server.address + ":" + &config.server.port.to_string()).parse()?;

    tracing::info!("Listening on {}", &addr);

    drop(init_guard);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
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
            .install_batch(opentelemetry_sdk::runtime::Tokio)?;

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

#[allow(dead_code)] // TODO: Replace when axum supports this again
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
