use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::PgPool;

use crate::route;

pub fn run(address: String, pool: web::Data<PgPool>) -> Server {
    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .route("/", web::get().to(route::greet))
            .route("/subscribe", web::post().to(route::subscribe))
    })
    .bind(address)
    .expect("failed to bind web port.")
    .run()
}
