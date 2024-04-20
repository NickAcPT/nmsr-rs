use axum::{
    extract::{ConnectInfo, MatchedPath},
    http::{header::USER_AGENT, Request},
    http::{HeaderMap, HeaderName},
};
use derive_more::Debug;
use opentelemetry::{
    global,
    propagation::Extractor,
    trace::TraceContextExt,
};
use std::net::SocketAddr;
use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::{DefaultOnBodyChunk, MakeSpan, OnFailure, OnRequest, OnResponse, TraceLayer},
};
use tracing::{
    field::{self, Empty},
    info_span,
};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::error::NmsrErrorExtension;

#[allow(clippy::declare_interior_mutable_const)]
const X_FORWARDED_FOR_HEADER: HeaderName = HeaderName::from_static("x-forwarded-for");
#[allow(clippy::declare_interior_mutable_const)]
const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
#[allow(clippy::declare_interior_mutable_const)]
const REFERER: HeaderName = HeaderName::from_static("referer");

pub struct NmsrTracing<B> {
    _phantom: std::marker::PhantomData<B>,
}

impl<B> NmsrTracing<B> {
    fn extract_header_as_str(headers: &HeaderMap, header: HeaderName) -> Option<String> {
        headers
            .get(header)
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string)
    }
}

impl<T> Clone for NmsrTracing<T> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<B> Default for NmsrTracing<B> {
    fn default() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}
type NmsrTraceLayer<B, R> = TraceLayer<
    SharedClassifier<ServerErrorsAsFailures>,
    NmsrTracing<B>,
    NmsrTracing<B>,
    NmsrTracing<R>,
    DefaultOnBodyChunk,
    (),
    NmsrTracing<R>,
>;

impl<B> NmsrTracing<B> {
    pub fn new_trace_layer<R>() -> NmsrTraceLayer<B, R> {
        TraceLayer::new_for_http()
            .make_span_with(Self::default())
            .on_request(Self::default())
            .on_response(NmsrTracing::default())
            .on_failure(NmsrTracing::default())
            .on_eos(())
    }
}

struct HeaderMapCarrier<'a>(&'a HeaderMap);

impl Extractor for HeaderMapCarrier<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(HeaderName::as_str).collect()
    }
}

impl<B> MakeSpan<B> for NmsrTracing<B> {
    fn make_span(&mut self, request: &Request<B>) -> tracing::Span {
        let user_agent = Self::extract_header_as_str(request.headers(), USER_AGENT)
            .unwrap_or_else(|| "<unknown>".to_string());

        let span = info_span!("HTTP request",
            http.path = Empty,
            http.method = ?request.method(),
            http.version = ?request.version(),
            http.user_agent = user_agent,
            http.client_ip = Empty,
            otel.kind = ?opentelemetry::trace::SpanKind::Server,
            http.status_code = Empty,
            http.referer = Empty,
            otel.status_code = Empty,
            trace_id = Empty,

            exception.message = Empty,

            request_id = Empty,
        );

        let context = global::get_text_map_propagator(|propagator| {
            propagator.extract(&HeaderMapCarrier(request.headers()))
        });

        if context.has_active_span() {
            span.set_parent(context);
        }

        span
    }
}

impl<B> OnRequest<B> for NmsrTracing<B> {
    fn on_request(&mut self, request: &Request<B>, span: &tracing::Span) {
        let path = request
            .extensions()
            .get::<MatchedPath>()
            .map_or(request.uri().path(), |p| p.as_str());

        let client_ip = Self::extract_header_as_str(request.headers(), X_FORWARDED_FOR_HEADER)
            .or_else(|| {
                request
                    .extensions()
                    .get::<ConnectInfo<SocketAddr>>()
                    .map(|ConnectInfo(c)| c.to_string())
            })
            .unwrap_or_else(|| "<unknown>".to_string());

        let request_id = Self::extract_header_as_str(request.headers(), X_REQUEST_ID)
            .unwrap_or_else(|| "<unknown>".to_string());

        span.record("http.path", path);
        span.record("http.client_ip", &client_ip);
        span.record("request_id", &request_id);

        let referer = Self::extract_header_as_str(request.headers(), REFERER);
        if let Some(referer) = referer {
            span.record("http.referer", &referer);
        }
    }
}

impl<B> OnResponse<B> for NmsrTracing<B> {
    fn on_response(
        self,
        response: &axum::http::Response<B>,
        _latency: std::time::Duration,
        span: &tracing::Span,
    ) {
        if response.status().is_client_error() || response.status().is_server_error() {
            if let Some(NmsrErrorExtension(original_error)) =
                response.extensions().get::<NmsrErrorExtension>()
            {
                span.record("exception.message", field::display(original_error));
            } else {
                span.record("exception.message", "Unknown error");
            }
        }

        span.record("http.status_code", response.status().as_u16());
        span.record("otel.status_code", "OK");
    }
}

impl<B, C: Debug> OnFailure<C> for NmsrTracing<B> {
    fn on_failure(
        &mut self,
        _failure_classification: C,
        _latency: std::time::Duration,
        span: &tracing::Span,
    ) {
        span.record("otel.status_code", "ERROR");
    }
}
