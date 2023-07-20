use std::future::{ready, Ready};

use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use tracing_actix_web::RequestId;

pub struct TraceIdHeader;

impl<S, B> Transform<S, ServiceRequest> for TraceIdHeader
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = TraceIdHeaderMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TraceIdHeaderMiddleware { service }))
    }
}

pub struct TraceIdHeaderMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for TraceIdHeaderMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let request_id = req.extensions().get::<RequestId>().copied();

        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            if let Some(request_id) = request_id {
                res.headers_mut().insert(
                    HeaderName::from_static("x-request-id"),
                    // this unwrap never fails, since UUIDs are valid ASCII strings
                    HeaderValue::from_str(&request_id.to_string()).unwrap(),
                );
            }
            Ok(res)
        })
    }
}
