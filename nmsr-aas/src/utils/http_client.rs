use axum::http::{HeaderName, HeaderValue};
use hyper::{
    body::{to_bytes, Bytes},
    Body, Client, Method, Request, Response,
};
use hyper_tls::HttpsConnector;
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

const USER_AGENT: &'static str = concat!(
    "NMSR-as-a-Service/",
    env!("VERGEN_GIT_SHA"),
    " (Discord=@nickacpt; +https://nmsr.nickac.dev/)"
);

pub struct NmsrHttpClient {
    inner: RwLock<
        SyncWrapper<
            BoxService<
                Request<Body>,
                Response<
                    ResponseBody<
                        Body,
                        NeverClassifyEos<ServerErrorsFailureClass>,
                        (),
                        (),
                        DefaultOnFailure,
                    >,
                >,
                hyper::Error,
            >,
        >,
    >,
}

impl NmsrHttpClient {
    pub fn new(rate_limit_per_second: u64) -> Self {
        create_http_client(rate_limit_per_second)
    }

    #[instrument(skip(self, parent_span), parent = parent_span)]
    pub(crate) async fn do_request(
        &self,
        url: &str,
        method: Method,
        parent_span: &Span,
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

        let body = response.into_body();

        to_bytes(body)
            .await
            .map_err(|e| MojangRequestError::BoxedRequestError(Box::new(e)))
    }
}

fn create_http_client(rate_limit_per_second: u64) -> NmsrHttpClient {
    let https = HttpsConnector::new();

    let client = Client::builder().build(https);

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
