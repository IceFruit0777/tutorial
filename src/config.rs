use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use sqlx::{postgres::PgPoolOptions, PgPool};

#[derive(serde::Deserialize)]
pub struct Config {
    pub web: WebConfig,
    pub database: DBConfig,
    pub email_client: EmailCientConfig,
}

#[derive(serde::Deserialize)]
pub struct WebConfig {
    pub host: String,
    port: u16,
}

impl WebConfig {
    pub fn server_address(&self) -> String {
        format!("{}:{}", &self.host, &self.port)
    }
}

#[derive(serde::Deserialize)]
pub struct DBConfig {
    host: String,
    port: u16,
    username: String,
    password: SecretString,
    db_name: String,
}

impl DBConfig {
    fn connect_str(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.db_name
        )
    }

    pub fn connect_str_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        )
    }

    pub fn connect(&self) -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(5))
            .connect_lazy(&self.connect_str())
            .expect("failed to create postgres connection pool.")
    }
}

#[derive(serde::Deserialize)]
pub struct EmailCientConfig {
    pub base_url: String,
    pub sender: String,
    pub authorization_token: SecretString,
    pub timeout_milliseconds: u64,
}

enum Enviroment {
    Local,
    Production,
}

impl Enviroment {
    fn as_str(&self) -> &str {
        match self {
            Enviroment::Local => "local",
            Enviroment::Production => "production",
        }
    }
}

impl TryFrom<String> for Enviroment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{other} is not a valid enviroment. \
                use `local` or `production` instead."
            )),
        }
    }
}

pub fn config() -> Config {
    // 获取根目录
    let base_path = std::env::current_dir().expect("failed to determine current directory.");
    let config_dir = base_path.join("config");
    // 获取环境变量`APP_ENVIROMENT`
    // 根据环境变量选择配置文件
    // `local` => 本地开发环境，local.yaml
    // `production` => 生产环境，production.yaml
    let env: Enviroment = std::env::var("APP_ENVIROMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("failed to parse env_var `APP_ENVIROMENT`");
    let env_filename = format!("{}.yaml", env.as_str());

    config::Config::builder()
        // 加载通用配置文件
        .add_source(config::File::from(config_dir.join("base.yaml")))
        // 加载当前环境配置文件
        .add_source(config::File::from(config_dir.join(env_filename)))
        .build()
        .expect("failed to read config.")
        .try_deserialize::<Config>()
        .expect("failed to deserialize config.")
}
