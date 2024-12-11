use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::routes;

pub fn run(listener: TcpListener, pool: web::Data<PgPool>) -> Server {
    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(pool.clone())
            .route("/", web::get().to(routes::greet))
            .route("/subscribe", web::post().to(routes::subscribe))
    })
    .listen(listener)
    .expect("failed to bind a TcpListener.")
    .run()
}
