use std::{net::TcpListener, time::Duration};

use actix_web::web;
use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tutorial::{email_client::EmailCient, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志系统
    telemetry::init_subscriber("tutorial");

    // 加载配置文件
    let config = web::Data::new(tutorial::config::config());
    // 初始化web监听器
    let listener = TcpListener::bind(format!("{}:{}", &config.web.host, &config.web.port))
        .expect("failed to bind web port.");
    // 初始化数据库连接池
    let pool = web::Data::new(
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(5))
            .connect_lazy_with(config.database.with_db()),
    );
    // 初始化邮件客户端
    let email_client = web::Data::new(EmailCient::from_config(&config));

    // web工作线程
    let web_task = tutorial::web_run(config, listener, pool.clone(), email_client.clone()).await?;
    let web_handler = web_task.handle();
    let web_task = tokio::spawn(web_task);
    // 发送邮件简报的工作线程
    let worker_task = tutorial::worker_run(pool, email_client);
    let worker_task = tokio::spawn(worker_task);

    // 优雅停机
    let signal = async {
        signal::ctrl_c().await.unwrap();
        println!("Received Ctrl+C, shuting down...");
    };
    let signal = tokio::spawn(signal);

    // 并行执行
    tokio::select! {
        _ = web_task => {},
        _ = worker_task => {},
        _ = signal => {
            web_handler.stop(true).await;
        },
    }

    Ok(())
}
