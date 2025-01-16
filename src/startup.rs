use std::net::TcpListener;

use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, dev::Server, middleware::from_fn, web, App, HttpServer};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
use secrecy::ExposeSecret;
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::{
    authentication::reject_anonymous_user, config::Config, email_client::EmailCient, routes,
};

pub async fn run(
    config: web::Data<Config>,
    listener: TcpListener,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailCient>,
) -> anyhow::Result<Server> {
    let secret_key = Key::from(config.web.hmac_secret.expose_secret().as_bytes());
    let cookie_msg_store = CookieMessageStore::builder(secret_key.clone()).build();
    let flash_msg_framework = FlashMessagesFramework::builder(cookie_msg_store).build();
    let redis_store = RedisSessionStore::new(config.redis_uri.expose_secret()).await?;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(flash_msg_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .wrap(TracingLogger::default())
            .route("/", web::get().to(routes::home))
            .route("/health_check", web::get().to(routes::health_check))
            .route("/login", web::get().to(routes::login_form))
            .route("/login", web::post().to(routes::login))
            .route("/subscribe", web::post().to(routes::subscribe))
            .route(
                "/subscription/confirm",
                web::get().to(routes::subscription_confirm),
            )
            .service(
                web::scope("/admin")
                    .wrap(from_fn(reject_anonymous_user))
                    .route("/dashboard", web::get().to(routes::admin_dashboard))
                    .route("/password", web::get().to(routes::change_password_form))
                    .route("/password", web::post().to(routes::change_password))
                    .route("/logout", web::post().to(routes::logout))
                    .route("/publish", web::get().to(routes::publish_form))
                    .route("/publish", web::post().to(routes::publish)),
            )
            .app_data(config.clone())
            .app_data(pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)
    .expect("failed to bind a TcpListener.")
    .run();

    Ok(server)
}
