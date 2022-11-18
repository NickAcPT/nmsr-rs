use actix_web::{get, HttpResponse, Responder};

#[get("/")]
pub(crate) async fn index() -> impl Responder {
    HttpResponse::Ok().body(include_str!("static/index.html")
        .replace("{{commit}}", env!("VERGEN_GIT_SHA")))
}
