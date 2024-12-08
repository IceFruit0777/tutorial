use tutorial::telemetry;

#[tokio::main]
async fn main() {
    telemetry::init_subscriber("tutorial");

    let config = tutorial::get_config();
    let address = format!("localhost:{}", config.web_port);
    let pool = config.database.connect().await;

    let _ = tutorial::run(address, pool).await;
}
