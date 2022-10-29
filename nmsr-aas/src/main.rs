mod mojang_requests;
mod routes;
mod utils;

use actix_web::{App, HttpServer};
use actix_web::middleware::Logger;
use routes::{index_route::index, get_skin_route::get_skin};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(get_skin)
            .wrap(Logger::default())
    }).bind(("0.0.0.0", 8080))?.run().await
}
