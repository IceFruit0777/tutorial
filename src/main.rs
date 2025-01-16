use std::{net::TcpListener, time::Duration};

use actix_web::web;
use sqlx::postgres::PgPoolOptions;
use tutorial::{email_client::EmailCient, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 遥测初始化
    telemetry::init_subscriber("tutorial");

    let config = web::Data::new(tutorial::config::config());

    let listener = TcpListener::bind(format!("{}:{}", &config.web.host, &config.web.port))
        .expect("failed to bind web port.");

    let pool = web::Data::new(
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(5))
            .connect_lazy_with(config.database.with_db()),
    );

    let email_client = web::Data::new(EmailCient::from_config(&config));

    // web工作线程
    let web_task = tutorial::web_run(config, listener, pool.clone(), email_client.clone()).await?;
    let web_task = tokio::spawn(web_task);
    // 发送邮件简报的工作线程
    let worker_task = tutorial::worker_run(pool, email_client);
    let worker_task = tokio::spawn(worker_task);
    // 并行执行
    tokio::select! {
        _ = web_task => {},
        _ = worker_task => {},
    }

    Ok(())
}
