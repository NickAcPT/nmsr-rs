use actix_web::body::MessageBody;
use actix_web::dev::ServiceRequest;
use actix_web::http::header;
use actix_web::{dev, Error};
use tracing::Span;
use tracing_actix_web::{DefaultRootSpanBuilder, RootSpanBuilder};

pub struct NMSRRootSpanBuilder;

impl RootSpanBuilder for NMSRRootSpanBuilder {
    fn on_request_start(request: &ServiceRequest) -> Span {
        let referer = request.request().headers().get(header::REFERER);
        if let Some(referer) = referer.and_then(|r| r.to_str().ok()) {
            tracing_actix_web::root_span!(request, referer)
        } else {
            tracing_actix_web::root_span!(request)
        }
    }

    fn on_request_end<B: MessageBody>(
        span: Span,
        outcome: &Result<dev::ServiceResponse<B>, Error>,
    ) {
        DefaultRootSpanBuilder::on_request_end(span, outcome);
    }
}
