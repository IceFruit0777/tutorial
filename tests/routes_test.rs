use std::net::TcpListener;

use actix_web::web;
use once_cell::sync::Lazy;
use sqlx::{Executor, PgPool};
use tutorial::telemetry;
use uuid::Uuid;

static TRACING: Lazy<()> = Lazy::new(|| telemetry::init_subscriber("test"));

async fn spawn_app() -> (String, web::Data<PgPool>) {
    Lazy::force(&TRACING);

    let config = tutorial::get_config();
    let address = format!("{}:{}", config.app.host, 0);
    let listener = TcpListener::bind(address).expect("failed to bind a random port.");
    let address = format!(
        "http://{}:{}",
        config.app.host,
        listener.local_addr().unwrap().port()
    );
    let pool = web::Data::new(connect_random_database().await);

    tokio::spawn(tutorial::run(listener, pool.clone()));

    (address, pool)
}

async fn connect_random_database() -> PgPool {
    let config = tutorial::get_config();

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

#[tokio::test]
async fn health_check() {
    let (address, _) = spawn_app().await;
    let client = reqwest::Client::new();

    let res = client
        .get(address)
        .send()
        .await
        .expect("failed to execute request.");
    assert!(res.status().is_success());
}

#[tokio::test]
async fn valid_subscribe() {
    let (address, pool) = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "name=IceFruit%20huang&email=git%40github.com";
    let res = client
        .post(format!("{address}/subscribe"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request.");
    assert_eq!(200, res.status().as_u16());

    let data = sqlx::query!("select name, email from subscription")
        .fetch_one(pool.get_ref())
        .await
        .expect("failed to execute query.");
    dbg!(&data);
    assert_eq!("IceFruit huang", data.name);
    assert_eq!("git@github.com", data.email);
}

#[tokio::test]
async fn invalid_subscribe() {
    let (address, _) = spawn_app().await;
    let client = reqwest::Client::new();

    let datas = [
        ("name=&email=git%40github01.com", "name is empty."),
        ("name=IceFruit%20huang&email=", "email is empty."),
    ];
    for (body, payload) in datas {
        let res = client
            .post(format!("{address}/subscribe"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request.");
        assert_eq!(400, res.status().as_u16(), "{payload}");
    }
}
