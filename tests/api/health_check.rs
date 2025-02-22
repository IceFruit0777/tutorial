use crate::helper::spawn_app;

#[tokio::test]
async fn health_check() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let url = app.web_base_url.join("/health_check").unwrap();
    let res = client
        .get(url)
        .send()
        .await
        .expect("failed to execute request.");
    assert!(res.status().is_success());
}
