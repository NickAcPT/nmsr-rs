use crate::error::{MojangRequestError, MojangRequestResult};
use axum::{
    body::Body,
    http::{HeaderName, HeaderValue},
    response::Response as AxumResponse,
};
use http::HeaderMap;
use http_body_util::BodyExt;
use hyper::{
    body::{Bytes, Incoming},
    Method, Request, Response,
};

use hyper_tls::HttpsConnector;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use std::{future::Future, pin::Pin, task::Poll, time::Duration, convert::Infallible};
use sync_wrapper::SyncWrapper;
use tokio::sync::RwLock;
use tower::{
    layer::util::Identity,
    service_fn,
    util::{BoxService, ServiceFn},
    Service, ServiceBuilder, ServiceExt,
};
use tower_http::{
    classify::{NeverClassifyEos, ServerErrorsFailureClass},
    set_header::SetRequestHeaderLayer,
    trace::{DefaultOnFailure, ResponseBody, TraceLayer},
};
use tracing::{instrument, Span};
use wasm_bindgen_futures::{
    future_to_promise,
    js_sys::{Object, Reflect, Uint8Array},
    JsFuture,
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm")]
use wasm_bindgen_futures::js_sys::Promise;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
extern "C" {
    fn do_request(url: &str, method: &str, headers: JsValue, body: Uint8Array) -> Promise;
}

const USER_AGENT: &str = concat!(
    "NMSR-as-a-Service/",
    env!("VERGEN_GIT_SHA"),
    " (Discord=@nickac; +https://nmsr.nickac.dev/)"
);

#[cfg(not(feature = "wasm"))]
pub(crate) type TraceResponseBody =
    ResponseBody<Incoming, NeverClassifyEos<ServerErrorsFailureClass>, (), (), DefaultOnFailure>;
#[cfg(not(feature = "wasm"))]
type BoxedTracedResponse =
    BoxService<Request<Body>, Response<TraceResponseBody>, hyper_util::client::legacy::Error>;

pub struct NmsrHttpClient {
    #[cfg(not(feature = "wasm"))]
    inner: RwLock<SyncWrapper<BoxedTracedResponse>>,
    #[cfg(feature = "wasm")]
    inner: RwLock<SyncWrapper<BoxService<axum::extract::Request, AxumResponse, JsError>>>,
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
    #[cfg(not(feature = "wasm"))]
    let client = {
        let https = HttpsConnector::new();

        // A new higher level client from hyper is in the works, so we gotta use the legacy one
        Client::builder(TokioExecutor::new()).build(https)
    };

    let client = {
        fn raw_do_request(
            uri: String,
            method: String,
            headers: HeaderMap,
            body: &[u8],
        ) -> NmsrRequestFuture {
            let body_array = Uint8Array::from(body);
            
            let mut headers_obj = Object::new();

            for (name, value) in headers.iter() {
                Reflect::set(
                    &mut headers_obj,
                    &name.as_str().into (),
                    &value.to_str().unwrap().into(),
                )
                .unwrap();
            }

            let promise = do_request(&uri, &method, headers_obj.into(), body_array);

            let promise = promise;
            let future = JsFuture::from(promise);

            NmsrRequestFuture(future)
        }

        async fn do_wasm_request(request: Request<Body>) -> Result<AxumResponse, JsError> {
            let (parts, body) = request.into_parts();

            let body = body.collect().await?.to_bytes();
            let uri = parts.uri.to_string();
            let method = parts.method.to_string();

            let result_bytes = { raw_do_request(uri, method, parts.headers, body.as_ref()) }.await?;

            let response = Response::builder().body(result_bytes.to_vec().into())?;

            Ok(response)
        }

        service_fn(do_wasm_request)
    };

    let tracing = {
        #[cfg(not(feature = "wasm"))]
        {
            TraceLayer::new_for_http().on_body_chunk(()).on_eos(())
        }
        #[cfg(feature = "wasm")]
        {
            Identity::new()
        }
    };
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

struct NmsrRequestFuture(JsFuture);

unsafe impl Send for NmsrRequestFuture {}

// Map future<jsvalue, jsvalue> to future<jsvalue, jserror>
impl Future for NmsrRequestFuture {
    type Output = Result<Vec<u8>, Infallible>;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let future = unsafe { self.map_unchecked_mut(|f| &mut f.0) };

        match future.poll(cx) {
            Poll::Ready(Ok(value)) => Poll::Ready(Ok(value
                .dyn_into::<Uint8Array>()
                .expect_throw("Failed to convert response to Uint8Array")
                .to_vec())),
            Poll::Ready(Err(err)) => panic!("Failed to do request: {:?}", err),
            Poll::Pending => Poll::Pending,
        }
    }
}
