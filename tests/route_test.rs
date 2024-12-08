use actix_web::web;
use once_cell::sync::Lazy;
use sqlx::PgPool;
use tutorial::telemetry;

static TRACING: Lazy<()> = Lazy::new(|| telemetry::init_subscriber("test"));

async fn spawn_web() -> web::Data<PgPool> {
    Lazy::force(&TRACING);

    let config = tutorial::get_config();
    let address = format!("localhost:{}", config.web_port);
    let pool = config.database.connect().await;

    tokio::spawn(tutorial::run(address, pool.clone()));

    pool
}

#[tokio::test]
async fn health_check() {
    let pool = spawn_web().await;
    let client = reqwest::Client::new();

    let res = client
        .get("http://localhost:8000")
        .send()
        .await
        .expect("failed to execute request.");
    assert!(res.status().is_success());

    let data = sqlx::query!("select name, email from subscription")
        .fetch_one(pool.get_ref())
        .await
        .expect("failed to execute query.");
    println!("{data:#?}");
    assert_eq!("IceFruit huang", data.name);
    assert_eq!("git@github.com", data.email);
}

#[tokio::test]
async fn subscribe() {
    let _ = spawn_web().await;
    let client = reqwest::Client::new();

    let body = "name=IceFruit%20huang&email=git%40github.com";
    let res = client
        .post("http://localhost:8000/subscribe")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request.");
    assert_eq!(500, res.status().as_u16());
}
