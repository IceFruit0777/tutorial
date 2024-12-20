use crate::helper::spawn_app;

#[tokio::test]
async fn health_check() {
    let (address, _, _) = spawn_app().await;
    let client = reqwest::Client::new();

    let res = client
        .get(&address)
        .send()
        .await
        .expect("failed to execute request.");
    assert!(res.status().is_success());
}
