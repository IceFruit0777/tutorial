#[tokio::main]
async fn main() {
    let config = tutorial::get_config();
    let address = format!("localhost:{}", config.web_port);
    let pool = config.database.connect().await;

    let _ = tutorial::run(address, pool).await;
}
