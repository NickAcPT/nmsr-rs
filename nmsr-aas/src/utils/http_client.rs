use axum::http::{HeaderName, HeaderValue};
use hyper::{body::HttpBody, client::HttpConnector, Body, Client, Method, Request, Response};
use hyper_tls::HttpsConnector;
use sync_wrapper::SyncWrapper;
use std::{error::Error, time::Duration};
use tokio::sync::RwLock;
use tower::{
    limit::RateLimit,
    util::{BoxLayer, BoxService, BoxCloneService},
    Service, ServiceBuilder, BoxError,
};
use tower_http::{
    classify::{
        NeverClassifyEos, ServerErrorsAsFailures, ServerErrorsFailureClass, SharedClassifier,
    },
    set_header::{SetRequestHeader, SetRequestHeaderLayer},
    trace::{
        DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, ResponseBody,
        Trace, TraceLayer,
    },
};

const USER_AGENT: &'static str = concat!(
    "NMSR-as-a-Service/",
    env!("VERGEN_GIT_SHA"),
    " (Discord=@nickacpt; +https://nmsr.nickac.dev/)"
);

pub(crate) fn create_http_client(rate_limit_per_second: u64) -> RwLock<SyncWrapper<BoxService<Request<Body>, Response<ResponseBody<Body, NeverClassifyEos<ServerErrorsFailureClass>, (), (), DefaultOnFailure>>, BoxError>>> {
    let https = HttpsConnector::new();

    let client = Client::builder().build(https);

    let tracing = TraceLayer::new_for_http().on_body_chunk(()).on_eos(());
    let service = ServiceBuilder::new()
        .boxed()
        .buffer(5)
        .rate_limit(rate_limit_per_second, Duration::from_secs(1))
        .layer(tracing)
        .layer(SetRequestHeaderLayer::overriding(
            HeaderName::from_static("user-agent"),
            HeaderValue::from_str(USER_AGENT).expect("Expected user-agent to be valid"),
        ))
        .service(client);

    RwLock::new(SyncWrapper::new(service))
}
