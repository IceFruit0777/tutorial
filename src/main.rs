use std::{net::TcpListener, time::Duration};

use sqlx::postgres::PgPoolOptions;
use tutorial::telemetry;

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

    let _ = tutorial::run(config, listener, pool).await;
}
