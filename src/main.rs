use std::net::TcpListener;

use actix_web::web;
use tutorial::{email_client::EmailCient, telemetry};

#[tokio::main]
async fn main() {
    // 遥测初始化
    telemetry::init_subscriber("tutorial");

    let config = tutorial::config::config();
    let listener =
        TcpListener::bind(config.web.server_address()).expect("failed to bind web port.");
    let pool = web::Data::new(config.database.connect());
    let email_client = web::Data::new(EmailCient::from_config(&config));

    let _ = tutorial::run(listener, pool, email_client).await;
}
