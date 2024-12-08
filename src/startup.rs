use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::route;

pub fn run(address: String, pool: web::Data<PgPool>) -> Server {
    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(pool.clone())
            .route("/", web::get().to(route::greet))
            .route("/subscribe", web::post().to(route::subscribe))
    })
    .bind(address)
    .expect("failed to bind web port.")
    .run()
}
