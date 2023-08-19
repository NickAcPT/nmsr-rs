mod config;
pub mod error;
mod model;
mod routes;
mod caching;

use tokio::main;

#[main]
async fn main() {
    println!("Hello, world!")
}
