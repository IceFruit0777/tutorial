use std::net::TcpListener;

use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use once_cell::sync::Lazy;
use reqwest::{Response, Url};
use serde_json::Value;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tutorial::{config::Config, telemetry};
use uuid::Uuid;
use wiremock::MockServer;

static TRACING: Lazy<()> = Lazy::new(|| telemetry::init_subscriber("test"));

pub struct TestApp {
    pub web_base_url: Url,
    pub pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
}

impl TestApp {
    pub async fn post_subscribe(&self, body: &str) -> Response {
        self.api_client
            .post(self.web_base_url.join("/subscribe").unwrap())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_string())
            .send()
            .await
            .unwrap()
    }

    pub async fn post_login(&self, body: &Value) -> Response {
        self.api_client
            .post(self.web_base_url.join("/login").unwrap())
            .form(body)
            .send()
            .await
            .unwrap()
    }

    pub async fn post_login_with_valid_user(&self) -> Response {
        let credential = serde_json::json!({
            "username": &self.test_user.username,
            "password": &self.test_user.password,
        });
        self.api_client
            .post(self.web_base_url.join("/login").unwrap())
            .form(&credential)
            .send()
            .await
            .unwrap()
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(self.web_base_url.join("/login").unwrap())
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    }

    pub async fn post_logout(&self) -> Response {
        self.api_client
            .post(self.web_base_url.join("/admin/logout").unwrap())
            .send()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(self.web_base_url.join("/admin/dashboard").unwrap())
            .send()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(self.web_base_url.join("/admin/password").unwrap())
            .send()
            .await
            .unwrap()
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_change_password(&self, body: &Value) -> Response {
        self.api_client
            .post(self.web_base_url.join("/admin/password").unwrap())
            .form(body)
            .send()
            .await
            .unwrap()
    }

    pub async fn post_publish(&self, body: &Value) -> Response {
        self.api_client
            .post(self.web_base_url.join("/admin/publish").unwrap())
            .form(&body)
            .send()
            .await
            .unwrap()
    }

    pub async fn post_publish_with_default_issue(&self) -> Response {
        let issue = serde_json::json!({
            "subject": "Publish Newsletter Test",
            "text_body": "This is someone called plain text.",
            "html_body": "<p>This is someone called html.</p>"
        });
        self.post_publish(&issue).await
    }

    /// 从发送订阅确认邮件的请求中提取确认链接
    pub async fn get_confirmation_link(&self) -> Url {
        let email_request = &self
            .email_server
            .received_requests()
            .await
            .unwrap()
            .pop()
            .unwrap();
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

pub struct TestUser {
    user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::default()
            .hash_password(&self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query!(
            r#"
            INSERT INTO users (user_id, username, password_hash)
            VALUES ($1, $2, $3)
            "#,
            &self.user_id,
            &self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .unwrap();
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

    tokio::spawn(tutorial::run(config, listener, pool.clone()).await.unwrap());

    let web_base_url = Url::parse(&web_base_url).unwrap();
    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();
    let app = TestApp {
        web_base_url,
        pool,
        email_server,
        test_user: TestUser::generate(),
        api_client,
    };

    // 添加随机测试管理员
    app.test_user.store(&app.pool).await;

    app
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

pub fn assert_is_redirect_to(res: &Response, redirect: &str) {
    assert_eq!(303, res.status().as_u16());
    assert_eq!(redirect, res.headers().get("Location").unwrap());
}
