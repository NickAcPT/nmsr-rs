pub mod model;
mod routes;
mod utils;

use crate::utils::tracing::NmsrTracing;
use crate::utils::tracing::NmsrTracingPropagatorLayer;

pub use utils::caching;
pub use utils::config;
pub use utils::error;

use std::{net::SocketAddr, time::Duration};

use axum::{
    http::Response,
    routing::get,
    Router, ServiceExt,
};
use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace, Resource},
    KeyValue,
};
use opentelemetry_otlp::{new_exporter, WithExportConfig};
use tokio::{main, signal};
use tower::ServiceBuilder;
use tower_http::{request_id::MakeRequestUuid, trace::TraceLayer, ServiceBuilderExt};
use tracing::{info, info_span, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[main]
async fn main() -> anyhow::Result<()> {
    setup_tracing()?;

    // build our application with a route
    let router = Router::new().route("/", get(root));

    let trace_layer = NmsrTracing::new_trace_layer();

    let app = ServiceBuilder::new()
        .set_x_request_id(MakeRequestUuid)
        .layer(trace_layer)
        .propagate_x_request_id()
        .layer(NmsrTracingPropagatorLayer)
        .service(router);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8621));

    tracing::info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn request_on_response<B>(response: &Response<B>, _latency: Duration, span: &Span) {
    span.record("otel.status_code", &response.status().as_u16());
}

const DEFAULT_FILTER: &'static str = "info,h2=off";

fn setup_tracing() -> anyhow::Result<()> {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or(DEFAULT_FILTER.into());

    let fmt_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_line_number(false)
        .with_file(false);

    global::set_text_map_propagator(TraceContextPropagator::new());

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(new_exporter().tonic().with_endpoint("http://[::1]:4317"))
        .with_trace_config(
            trace::config().with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                "nmsr-aas",
            )])),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

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
