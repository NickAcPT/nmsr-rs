use actix_web::{get, head, HttpResponse, Responder};
use reqwest::header::CONTENT_TYPE;

pub const INDEX_HTML: &str = include_str!("static/index.html");
pub const VERGEN_SHA: &str = env!("VERGEN_GIT_SHA");

#[get("/")]
pub(crate) async fn index() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(INDEX_HTML
        .replace("{{commit}}", VERGEN_SHA))
}

#[head("/")]
pub(crate) async fn index_head() -> impl Responder {
    HttpResponse::Ok().append_header((CONTENT_TYPE, "text/html; charset=utf-8")).finish()
}