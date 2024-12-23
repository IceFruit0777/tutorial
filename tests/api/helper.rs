use std::net::TcpListener;

use once_cell::sync::Lazy;
use reqwest::{Response, Url};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tutorial::{config::Config, telemetry};
use uuid::Uuid;
use wiremock::MockServer;

static TRACING: Lazy<()> = Lazy::new(|| telemetry::init_subscriber("test"));

pub struct TestApp {
    pub web_base_url: Url,
    pub pool: PgPool,
    pub email_server: MockServer,
}

impl TestApp {
    /// 发送订阅请求
    pub async fn subscribe_request(&self, body: &'static str) -> Response {
        let client = reqwest::Client::new();
        client
            .post(self.web_base_url.join("/subscribe").unwrap())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request.")
    }

    /// 从发送订阅确认邮件的请求中提取确认链接
    pub async fn get_confirmation_links(&self) -> Url {
        let email_request = &self.email_server.received_requests().await.unwrap()[0];
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let confirmation_link = Url::parse(&raw_link).unwrap();
            assert_eq!("127.0.0.1", confirmation_link.host_str().unwrap());
            confirmation_link
        };

        let text_link = get_link(&body["TextBody"].as_str().unwrap());
        let html_link = get_link(&body["HtmlBody"].as_str().unwrap());
        assert_eq!(text_link, html_link);

        text_link
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let mut config = tutorial::config::config();
    let address = format!("{}:{}", &config.web.host, 0);
    let listener = TcpListener::bind(&address).expect("failed to bind web port.");
    let pool = connect_random_database(&mut config).await;

    // 模拟邮件服务器
    let email_server = MockServer::start().await;
    config.email_client.base_url = email_server.uri();

    // 获取绑定的随机端口
    let port = listener.local_addr().unwrap().port();
    // 设置web base url
    let web_base_url = format!("http://{}:{}", &config.web.host, &port);
    config.web.base_url = web_base_url.clone();

    tokio::spawn(tutorial::run(config, listener, pool.clone()));

    let web_base_url = Url::parse(&web_base_url).unwrap();
    TestApp {
        web_base_url,
        pool,
        email_server,
    }
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
