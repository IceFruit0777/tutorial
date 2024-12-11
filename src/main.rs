use std::net::TcpListener;

use tutorial::telemetry;

#[tokio::main]
async fn main() {
    telemetry::init_subscriber("tutorial");

    let config = tutorial::get_config();
    let address = format!("{}:{}", config.app.host, config.app.port);
    let listener = TcpListener::bind(address).expect("failed to bind web port.");
    let pool = config.database.connect();

    let _ = tutorial::run(listener, pool).await;
}
