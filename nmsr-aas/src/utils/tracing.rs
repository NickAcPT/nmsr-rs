use axum::{
    extract::{ConnectInfo, MatchedPath},
    http::{header::USER_AGENT, HeaderValue},
    http::{HeaderMap, HeaderName},
};
use derive_more::{Debug, Deref};
use opentelemetry::{
    global,
    propagation::{Extractor, Injector},
};
use std::net::SocketAddr;
use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::{
        DefaultOnBodyChunk, MakeSpan, OnFailure, OnRequest,
        OnResponse, TraceLayer,
    },
};
use tracing::{field::Empty, info_span, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

const X_FORWARDED_FOR_HEADER: HeaderName = HeaderName::from_static("x-forwarded-for");
const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

pub struct NmsrTracing<B> {
    _phantom: std::marker::PhantomData<B>,
}

impl<B> NmsrTracing<B> {
    fn extract_header_as_str(headers: &HeaderMap, header: HeaderName) -> Option<String> {
        headers
            .get(header)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }
}

impl<T> Clone for NmsrTracing<T> {
    fn clone(&self) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<B> Default for NmsrTracing<B> {
    fn default() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<B> NmsrTracing<B> {
    pub fn new_trace_layer<R>() -> TraceLayer<
        SharedClassifier<ServerErrorsAsFailures>,
        NmsrTracing<B>,
        NmsrTracing<B>,
        NmsrTracing<R>,
        DefaultOnBodyChunk,
        (),
        NmsrTracing<R>,
    > {
        TraceLayer::new_for_http()
            .make_span_with(NmsrTracing::default())
            .on_request(NmsrTracing::default())
            .on_response(NmsrTracing::default())
            .on_failure(NmsrTracing::default())
            .on_eos(())
    }
}

struct HeaderMapCarrier<'a>(&'a HeaderMap);
struct MutableHeaderMapCarrier<'a>(&'a mut HeaderMap);

impl Extractor for HeaderMapCarrier<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

impl<'a> Injector for MutableHeaderMapCarrier<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(
            HeaderName::from_lowercase(key.as_bytes()).expect("Invalid header name"),
            HeaderValue::from_str(&value).expect("Invalid header value"),
        );
    }
}

impl<B> MakeSpan<B> for NmsrTracing<B> {
    fn make_span(&mut self, request: &axum::http::Request<B>) -> tracing::Span {
        let user_agent = Self::extract_header_as_str(request.headers(), USER_AGENT)
            .unwrap_or("<unknown>".to_string());

        let span = info_span!("HTTP request",
            http.path = Empty,
            http.method = ?request.method(),
            http.version = ?request.version(),
            http.user_agent = user_agent,
            http.client_ip = Empty,
            otel.name = Empty,
            otel.kind = ?opentelemetry::trace::SpanKind::Server,
            otel.status_code = Empty,

            exception.message = Empty,

            trace_id = Empty,
            request_id = Empty,
        );

        global::get_text_map_propagator(|propagator| {
            span.set_parent(propagator.extract(&HeaderMapCarrier(&request.headers())));
        });

        span
    }
}

impl<B> OnRequest<B> for NmsrTracing<B> {
    fn on_request(&mut self, request: &axum::http::Request<B>, span: &tracing::Span) {
        let path = request
            .extensions()
            .get::<MatchedPath>()
            .map(|p| p.as_str())
            .unwrap_or(request.uri().path());

        let client_ip = Self::extract_header_as_str(request.headers(), X_FORWARDED_FOR_HEADER)
            .or_else(|| {
                request
                    .extensions()
                    .get::<ConnectInfo<SocketAddr>>()
                    .map(|ConnectInfo(c)| c.to_string())
            })
            .unwrap_or("<unknown>".to_string());

        let request_id = Self::extract_header_as_str(request.headers(), X_REQUEST_ID)
            .unwrap_or("<unknown>".to_string());

        span.record("http.path", &path);
        span.record("http.client_ip", &client_ip);
        span.record("request_id", &request_id);
    }
}

impl<B> OnResponse<B> for NmsrTracing<B> {
    fn on_response(
        self,
        response: &axum::http::Response<B>,
        _latency: std::time::Duration,
        span: &tracing::Span,
    ) {
        {
            type AnyMap = std::collections::HashMap<std::any::TypeId, Box<dyn std::any::Any + Send + Sync>>;
            struct ExtensionsMine {
                // If extensions are never used, no need to carry around an empty HashMap.
                // That's 3 words. Instead, this is only 1 word.
                map: Option<Box<AnyMap>>,
            }
            
            let ext = response.extensions().clone();
            let ext = unsafe {
                std::mem::transmute::<&axum::http::Extensions, &ExtensionsMine>(&ext)
            };
            
            if let Some(owomap) = ext.map.as_deref() {
                for (type_id, any) in owomap.deref() {
                    println!("type_id: {:?}", type_id);
                    
                    if let Some(any) = any.downcast_ref::<String>() {
                        println!("any: {:?}", any);
                    }
                    
                }
            }
            
            println!("ext: {:?}", ext.map)
            
        }
        
        span.record("otel.status_code", &response.status().as_u16());
    }
}

impl<B, C: Debug> OnFailure<C> for NmsrTracing<B> {
    fn on_failure(
        &mut self,
        failure_classification: C,
        _latency: std::time::Duration,
        span: &tracing::Span,
    ) {
        span.record(
            "exception.message",
            format!("{:?}", failure_classification).as_str(),
        );
    }
}