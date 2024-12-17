use std::net::TcpListener;

use actix_web::web;
use once_cell::sync::Lazy;
use sqlx::{Executor, PgPool};
use tutorial::{config::Config, email_client::EmailCient, telemetry};
use uuid::Uuid;

static TRACING: Lazy<()> = Lazy::new(|| telemetry::init_subscriber("test"));

pub async fn spawn_app() -> (String, web::Data<PgPool>) {
    Lazy::force(&TRACING);

    let config = tutorial::config::config();
    let address = format!("{}:{}", config.web.host, 0);
    let listener = TcpListener::bind(&address).expect("failed to bind web port.");
    let pool = web::Data::new(connect_random_database(&config).await);
    let email_client = web::Data::new(EmailCient::from_config(&config));

    // 获取绑定的随机端口
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://{}:{}", &config.web.host, &port);

    tokio::spawn(tutorial::run(listener, pool.clone(), email_client));

    (address, pool)
}

async fn connect_random_database(config: &Config) -> PgPool {
    // 连接postgres实例
    let pool = PgPool::connect(&config.database.connect_str_without_db())
        .await
        .expect("failed connect to postgres.");
    // 生成随机数据库名称
    let db_name = Uuid::new_v4().to_string();
    // 创建随机数据库
    pool.execute(format!(r#"CREATE DATABASE "{}";"#, db_name).as_str())
        .await
        .expect("failed to create database.");
    // 连接数据库
    let pool = PgPool::connect(
        format!("{}/{}", config.database.connect_str_without_db(), db_name).as_str(),
    )
    .await
    .expect("failed connect to random database.");
    // 执行数据库迁移
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to migrate database.");

    pool
}
