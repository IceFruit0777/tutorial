use secrecy::{ExposeSecret, SecretString};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions,
};

#[derive(serde::Deserialize)]
pub struct Config {
    pub web: WebConfig,
    pub database: DBConfig,
    pub email_client: EmailCientConfig,
}

#[derive(serde::Deserialize)]
pub struct WebConfig {
    pub host: String,
    pub base_url: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
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
    #[serde(deserialize_with = "deserialize_number_from_string")]
    port: u16,
    username: String,
    password: SecretString,
    pub db_name: String,
    require_ssl: bool,
}

impl DBConfig {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .username(&self.username)
            .password(self.password.expose_secret())
            .host(&self.host)
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        PgConnectOptions::log_statements(
            self.without_db().database(&self.db_name),
            tracing::log::LevelFilter::Trace,
        )
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
        // 从环境变量中加载配置，获取动态配置或敏感信息
        // Example:
        //     `APP_WEB__PORT=8000` => `config.web.port=8000`
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()
        .expect("failed to read config.")
        .try_deserialize::<Config>()
        .expect("failed to deserialize config.")
}
