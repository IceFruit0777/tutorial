use std::{net::TcpListener, time::Duration};

use actix_web::web;
use sqlx::postgres::PgPoolOptions;
use tutorial::{email_client::EmailCient, telemetry};

#[tokio::main]
async fn main() {
    // 遥测初始化
    telemetry::init_subscriber("tutorial");

    let config = tutorial::config::config();
    let listener =
        TcpListener::bind(config.web.server_address()).expect("failed to bind web port.");
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(5))
        .connect_lazy_with(config.database.with_db());

    // 构造web Arc
    let pool = web::Data::new(pool);
    let email_client = web::Data::new(EmailCient::from_config(&config));

    let _ = tutorial::run(listener, pool, email_client).await;
}
