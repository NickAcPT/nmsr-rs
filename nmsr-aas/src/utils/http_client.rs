use axum::{
    body::Body,
    http::{HeaderName, HeaderValue},
};
use http_body_util::BodyExt;
use hyper::{body::{Bytes, Incoming}, Method, Request, Response};
use hyper_tls::HttpsConnector;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use std::time::Duration;
use sync_wrapper::SyncWrapper;
use tokio::sync::RwLock;
use tower::{util::BoxService, Service, ServiceBuilder, ServiceExt};
use tower_http::{
    classify::{NeverClassifyEos, ServerErrorsFailureClass},
    set_header::SetRequestHeaderLayer,
    trace::{DefaultOnFailure, ResponseBody, TraceLayer},
};
use tracing::{instrument, Span};

use crate::error::{MojangRequestError, MojangRequestResult};

const USER_AGENT: &str = concat!(
    "NMSR-as-a-Service/",
    env!("VERGEN_GIT_SHA"),
    " (Discord=@nickac; +https://nmsr.nickac.dev/)"
);

pub(crate) type TraceResponseBody =
    ResponseBody<Incoming, NeverClassifyEos<ServerErrorsFailureClass>, (), (), DefaultOnFailure>;
type BoxedTracedResponse = BoxService<Request<Body>, Response<TraceResponseBody>, hyper_util::client::legacy::Error>;

pub struct NmsrHttpClient {
    inner: RwLock<SyncWrapper<BoxedTracedResponse>>,
}

impl NmsrHttpClient {
    pub fn new(rate_limit_per_second: u64) -> Self {
        create_http_client(rate_limit_per_second)
    }

    #[allow(clippy::significant_drop_tightening)] // Not worth making the code less readable
    #[instrument(skip(self, parent_span, on_error), parent = parent_span)]
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
            .body(Body::empty())?;

        let response = {
            let mut client = self.inner.write().await;
            let service = client.get_mut().ready().await?;

            service.call(request).await?
        };

        if !response.status().is_success() {
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

fn create_http_client(rate_limit_per_second: u64) -> NmsrHttpClient {
    let https = HttpsConnector::new();
    
    // A new higher level client from hyper is in the works, so we gotta use the legacy one
    let client = Client::builder(TokioExecutor::new()).build(https);

    let tracing = TraceLayer::new_for_http().on_body_chunk(()).on_eos(());
    let service = ServiceBuilder::new()
        .boxed()
        .rate_limit(rate_limit_per_second, Duration::from_secs(1))
        .layer(tracing)
        .layer(SetRequestHeaderLayer::overriding(
            HeaderName::from_static("user-agent"),
            HeaderValue::from_str(USER_AGENT).expect("Expected user-agent to be valid"),
        ))
        .service(client);

    NmsrHttpClient {
        inner: RwLock::new(SyncWrapper::new(service)),
    }
}
