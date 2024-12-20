use std::net::TcpListener;

use actix_web::web;
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tutorial::{config::Config, email_client::EmailCient, telemetry};
use uuid::Uuid;
use wiremock::MockServer;

static TRACING: Lazy<()> = Lazy::new(|| telemetry::init_subscriber("test"));

pub async fn spawn_app() -> (String, web::Data<PgPool>, MockServer) {
    Lazy::force(&TRACING);

    let mut config = tutorial::config::config();
    let address = format!("{}:{}", &config.web.host, 0);
    let listener = TcpListener::bind(&address).expect("failed to bind web port.");
    let pool = web::Data::new(connect_random_database(&mut config).await);

    // 模拟邮件服务器
    let email_server = MockServer::start().await;
    config.email_client.base_url = email_server.uri();
    let email_client = web::Data::new(EmailCient::from_config(&config));

    // 获取绑定的随机端口
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://{}:{}", &config.web.host, &port);

    tokio::spawn(tutorial::run(listener, pool.clone(), email_client));

    (address, pool, email_server)
}

async fn connect_random_database(config: &mut Config) -> PgPool {
    // 连接postgres实例
    let mut connection = PgConnection::connect_with(&config.database.without_db())
        .await
        .expect("failed connect to postgres.");
    // 生成随机数据库名称
    config.database.db_name = Uuid::new_v4().to_string();
    // 创建随机数据库
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, &config.database.db_name).as_str())
        .await
        .expect("failed to create database.");
    // 连接数据库
    let pool = PgPool::connect_with(config.database.with_db())
        .await
        .expect("failed connect to database.");
    // 执行数据库迁移
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to migrate database.");

    pool
}
