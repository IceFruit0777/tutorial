use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::{config::Config, email_client::EmailCient, routes};

pub fn run(config: Config, listener: TcpListener, pool: PgPool) -> Server {
    let config = web::Data::new(config);
    let pool = web::Data::new(pool);
    let email_client = web::Data::new(EmailCient::from_config(&config));

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(config.clone())
            .app_data(pool.clone())
            .app_data(email_client.clone())
            .route("/health_check", web::get().to(routes::health_check))
            .route("/subscribe", web::post().to(routes::subscribe))
            .route("/newsletter/publish", web::post().to(routes::publish))
            .route(
                "/subscription/confirm",
                web::get().to(routes::subscription_confirm),
            )
    })
    .listen(listener)
    .expect("failed to bind a TcpListener.")
    .run()
}
