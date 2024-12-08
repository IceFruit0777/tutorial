use actix_web::web;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct Config {
    pub database: DBConfig,
    pub web_port: u16
}

#[derive(serde::Deserialize)]
pub struct DBConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
    db_name: String
}

impl DBConfig {
    fn connect_str(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.db_name
        )
    }

    pub async fn connect(&self) -> web::Data<PgPool> {
        web::Data::new(
            PgPool::connect(&self.connect_str())
            .await
            .expect("failed connect to postgres.")
        )
    }
}

pub fn get_config() -> Config {
    config::Config::builder()
        .add_source(config::File::new("config.yaml", config::FileFormat::Yaml))
        .build()
        .expect("failed to read config.yaml.")
        .try_deserialize::<Config>()
        .expect("failed to deserialize config.yaml.")
}
