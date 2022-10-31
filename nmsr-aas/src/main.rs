mod mojang_requests;
mod routes;
mod utils;

use crate::{utils::errors::NMSRaaSError, utils::Result};
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use nmsr_lib::parts::manager::PartsManager;
use routes::{get_skin_route::get_skin, index_route::index};

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let parts_manager = PartsManager::new("parts")?;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(Data::new(parts_manager.clone()))
            .service(index)
            .service(get_skin)
    });
    println!("Starting server on http://127.0.0.1:8080");
    server
        .bind(("0.0.0.0", 8080))
        .map_err(NMSRaaSError::IOError)?
        .run()
        .await
        .map_err(NMSRaaSError::IOError)
}
