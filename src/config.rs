use std::time::Duration;

use actix_web::web;
use secrecy::{ExposeSecret, SecretString};
use sqlx::{postgres::PgPoolOptions, PgPool};

#[derive(serde::Deserialize)]
pub struct Config {
    pub database: DBConfig,
    pub app: App,
}

#[derive(serde::Deserialize)]
pub struct App {
    pub host: String,
    pub port: u16,
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

    pub fn connect(&self) -> web::Data<PgPool> {
        web::Data::new(
            PgPoolOptions::new()
                .acquire_timeout(Duration::from_secs(5))
                .connect_lazy(&self.connect_str())
                .expect("failed to create postgres connection pool."),
        )
    }
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

pub fn get_config() -> Config {
    let base_path = std::env::current_dir().expect("failed to determine current directory.");
    let config_dir = base_path.join("config");
    let env: Enviroment = std::env::var("APP_ENVIROMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("failed to parse APP_ENVIROMENT");
    let env_filename = format!("{}.yaml", env.as_str());

    config::Config::builder()
        .add_source(config::File::from(config_dir.join("base.yaml")))
        .add_source(config::File::from(config_dir.join(env_filename)))
        .build()
        .expect("failed to read config.")
        .try_deserialize::<Config>()
        .expect("failed to deserialize config.")
}
