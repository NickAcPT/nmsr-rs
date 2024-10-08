use axum::http::{HeaderName, HeaderValue};
use http::StatusCode;
use http_body_util::{BodyExt, Empty};
use hyper::{body::Bytes, Method, Request};
use hyper_tls::{native_tls::TlsConnector, HttpsConnector};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use std::{
    future::{ready, Ready},
    time::Duration,
};
use tower::{
    buffer::Buffer,
    limit::RateLimit,
    retry::{Policy, Retry},
    timeout::{Timeout, TimeoutLayer},
    Service, ServiceBuilder, ServiceExt,
};
use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    set_header::{SetRequestHeader, SetRequestHeaderLayer},
    trace::{
        DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, Trace, TraceLayer,
    },
};
use tracing::{instrument, Span};

use crate::error::{MojangRequestError, MojangRequestResult};

const USER_AGENT: &str = concat!(
    "NMSR-as-a-Service/",
    env!("VERGEN_IS_LITERALLY_TRASH__IT_DOES_NOT_WORK_AND_IT_ACTUALLY_BREAKS_EVERY_TIME_I_UPDATE_IT__LIKE_SERIOUSLY_HOW_IS_THAT_POSSIBLE___STOP_CHANGING_THE_DAMN_IMPLEMENTATION___I_JUST_WANT_A_STUPID_GIT_HASH"),
    " (Discord=@nickac; +https://nmsr.nickac.dev/)"
);

pub(crate) type SyncBody =
    http_body_util::combinators::BoxBody<Bytes, hyper_util::client::legacy::Error>;

pub(crate) type SyncBodyClient = Client<HttpsConnector<HttpConnector>, SyncBody>;

pub(crate) type NmsrTraceLayer = Trace<
    SetRequestHeader<SyncBodyClient, HeaderValue>,
    SharedClassifier<ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
    (),
    (),
    DefaultOnFailure,
>;

pub struct NmsrHttpClient {
    inner: Buffer<Request<SyncBody>, <RateLimit<Retry<MojankRetryPolicy, Timeout<NmsrTraceLayer>>> as Service<Request<SyncBody>>>::Future>,
}

impl NmsrHttpClient {
    pub fn new(rate_limit_per_second: u64, request_timeout_seconds: u64) -> Self {
        create_http_client(rate_limit_per_second, request_timeout_seconds)
    }

    #[instrument(skip(self, parent_span, on_error), parent = parent_span, err)]
    pub(crate) async fn do_request(
        &self,
        url: &str,
        method: Method,
        parent_span: &Span,
        on_error: impl FnOnce() -> Option<MojangRequestError>,
    ) -> MojangRequestResult<Bytes> {
        let request = Request::builder()
            .method(method)
            .uri(url)
            .body(SyncBody::new(Empty::new().map_err(|e| {
                unreachable!("Empty body should not error: {}", e)
            })))?;

        let response = {
            let mut svc = self.inner.clone();

            let service = svc
                .ready()
                .await
                .map_err(MojangRequestError::BoxedRequestError)?;

            service
                .call(request)
                .await
                .map_err(MojangRequestError::BoxedRequestError)?
        };

        if response.status() != StatusCode::OK {
            if let Some(err) = on_error() {
                return Err(err);
            }
        }

        let body = response.into_body();

        body.collect()
            .await
            .map(|b| b.to_bytes())
            .map_err(|e| MojangRequestError::BoxedRequestError(Box::new(e)))
    }
}

fn create_http_client(rate_limit_per_second: u64, request_timeout_seconds: u64) -> NmsrHttpClient {
    let mut http = HttpConnector::new();
    http.set_nodelay(true);
    http.enforce_http(false);

    let tls = TlsConnector::new().expect("Expected TLS connector to be valid");

    let https = HttpsConnector::from((http, tls.into()));

    // A new higher level client from hyper is in the works, so we gotta use the legacy one
    let client = Client::builder(TokioExecutor::new()).build(https);

    let tracing = TraceLayer::new_for_http().on_body_chunk(()).on_eos(());

    let service = ServiceBuilder::new()
        .buffer(rate_limit_per_second.saturating_mul(2) as usize)
        .rate_limit(rate_limit_per_second, Duration::from_secs(1))
        .layer(CloneRetryLayer::new(MojankRetryPolicy::new(
            5, /* Retry attempts */
        )))
        .layer(TimeoutLayer::new(Duration::from_secs(
            request_timeout_seconds,
        )))
        .layer(tracing)
        .layer(SetRequestHeaderLayer::overriding(
            HeaderName::from_static("user-agent"),
            HeaderValue::from_str(USER_AGENT).expect("Expected user-agent to be valid"),
        ))
        .check_clone()
        .service(client);

    NmsrHttpClient { inner: service }
}

#[derive(Copy, Clone, Debug)]
struct MojankRetryPolicy {
    attempts: usize,
}

impl MojankRetryPolicy {
    pub fn new(attempts: usize) -> Self {
        Self { attempts }
    }
}

impl<P, Res> Policy<Request<SyncBody>, Res, P> for MojankRetryPolicy {
    type Future = Ready<()>;

    fn retry(&mut self, _req: &mut Request<SyncBody>, result: &mut Result<Res, P>) -> Option<Self::Future> {
        match result {
            Ok(_) => None,
            Err(_) => {
                if self.attempts > 0 {
                    self.attempts -= 1;
                    Some(ready(()))
                } else {
                    None
                }
            }
        }
    }

    fn clone_request(&mut self, req: &Request<SyncBody>) -> Option<Request<SyncBody>> {
        let method = req.method().clone();
        let uri = req.uri().clone();

        let mut builder = Request::builder().method(method).uri(uri);

        for (key, value) in req.headers() {
            builder = builder.header(key, value);
        }

        builder
            .body(SyncBody::new(Empty::new().map_err(|e| {
                unreachable!("Empty body should not error: {}", e)
            })))
            .ok()
    }
}

/// Clone retry layer
#[derive(Clone, Debug)]
pub struct CloneRetryLayer<P> {
    policy: P,
}

impl<P> CloneRetryLayer<P> {
    /// Create a new [`CloneRetryLayer`] from a retry policy
    pub fn new(policy: P) -> Self {
        CloneRetryLayer { policy }
    }
}

impl<P, S> tower::Layer<S> for CloneRetryLayer<P>
where
    P: Clone,
{
    type Service = Retry<P, S>;

    fn layer(&self, service: S) -> Self::Service {
        let policy = self.policy.clone();
        Retry::new(policy, service)
    }
}
