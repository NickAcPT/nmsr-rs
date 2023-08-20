mod caching;
mod config;
pub mod error;
mod model;
mod routes;

use std::{net::SocketAddr, time::Duration};

use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, MatchedPath},
    http::{header::USER_AGENT, HeaderMap, HeaderName, Request, Response, HeaderValue},
    routing::get, Router, ServiceExt,
};
use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace, Resource},
    KeyValue, propagation::{Extractor, Injector},
};
use opentelemetry_otlp::{new_exporter, WithExportConfig};
use tokio::{main, signal};
use tower::ServiceBuilder;
use tower_http::{
    trace::TraceLayer,
    ServiceBuilderExt, request_id::MakeRequestUuid,
};
use tracing::{trace_span, Span, info_span, info};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{
    util::SubscriberInitExt, layer::SubscriberExt,
};

#[main]
async fn main() -> anyhow::Result<()> {
    setup_tracing()?;

    // build our application with a route
    let router = Router::new().route("/", get(root));

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(request_make_span)
        .on_response(request_on_response)
        .on_eos(());

    let app = ServiceBuilder::new()
        .set_x_request_id(MakeRequestUuid)
        .layer(trace_layer)
        .propagate_x_request_id()
        .service(router);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8621));

    tracing::info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

const X_FORWARDED_FOR_HEADER: HeaderName = HeaderName::from_static("x-forwarded-for");
const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

fn request_make_span(request: &Request<Body>) -> Span {
    fn extract_header_as_str(headers: &HeaderMap, header: HeaderName) -> Option<String> {
        headers
            .get(header)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }

    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str())
        .unwrap_or(request.uri().path());

    let headers = request.headers();
    let request_id =
        extract_header_as_str(headers, X_REQUEST_ID).unwrap_or("<unknown>".to_string());
    let user_agent = extract_header_as_str(headers, USER_AGENT).unwrap_or("<unknown>".to_string());
    let client_ip = extract_header_as_str(headers, X_FORWARDED_FOR_HEADER)
        .or_else(|| {
            request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ConnectInfo(c)| c.to_string())
        })
        .unwrap_or("<unknown>".to_string());

    let span = info_span!("HTTP request",
        http.method = ?request.method(),
        http.path = path,
        http.version = ?request.version(),
        http.user_agent = user_agent,
        http.client_ip = client_ip,
        request_id = request_id,
        otel.name = tracing::field::Empty,
        otel.kind = ?opentelemetry::trace::SpanKind::Server,
        otel.status_code = tracing::field::Empty,
    );
    
    struct HeaderMapCarrier<'a>(&'a HeaderMap);
    
    impl Extractor for HeaderMapCarrier<'_> {
        fn get(&self, key: &str) -> Option<&str> {
            self.0.get(key).and_then(|v| v.to_str().ok())
        }

        fn keys(&self) -> Vec<&str> {
            self.0.keys().map(|k| k.as_str()).collect()
        }
    }
    
    global::get_text_map_propagator(|propagator| {
        let carrier = &HeaderMapCarrier(&request.headers());
        
        let context = propagator.extract(carrier);
        span.set_parent(context);
    });
    
    span
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
