use axum::{
    extract::{ConnectInfo, MatchedPath},
    http::{header::USER_AGENT, HeaderValue, Request, Response},
    http::{HeaderMap, HeaderName},
};
use derive_more::Debug;
use opentelemetry::{
    global,
    propagation::{Extractor, Injector},
};
use tower::{Layer, Service};
use std::{net::SocketAddr, future::Future, pin::Pin, task::{Context, Poll}};
use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::{
        DefaultOnBodyChunk, DefaultOnRequest, DefaultOnResponse, MakeSpan, OnFailure, OnRequest,
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


#[derive(Clone, Debug)]
pub struct NmsrTracingPropagatorLayer;

impl<S> Layer<S> for NmsrTracingPropagatorLayer {
    type Service = NmsrTracingPropagatorService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        NmsrTracingPropagatorService { inner }
    }
}

#[derive(Clone, Debug)]
pub struct NmsrTracingPropagatorService<S> {
    inner: S,
}

impl<S, B, B2> Service<Request<B>> for NmsrTracingPropagatorService<S>
where
    S: Service<Request<B>, Response = Response<B2>> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Response<B2>, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    #[allow(unused_mut)]
    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        let future = self.inner.call(request);

        let result = Box::pin(async move {
            let mut response = future.await?;
        
            let mut headers = response.headers_mut();
            
            let context = Span::current().context();
            
            global::get_text_map_propagator(|propagator| {
                propagator.inject_context(&context, &mut MutableHeaderMapCarrier(&mut headers));
            });
            
            Ok(response)
        });
        
        result
    }
}