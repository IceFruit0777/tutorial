use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helper::{spawn_app, TestApp};

async fn create_unconfirmed_subscriber(app: &TestApp, email: &str) -> reqwest::Url {
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&app.email_server)
        .await;

    let body = format!("name=IceFruit%20huang&email={email}");
    app.subscribe_request(&body)
        .await
        .error_for_status()
        .unwrap();

    app.get_confirmation_link().await
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let email = "git@github.com";
    let link = create_unconfirmed_subscriber(app, email).await;
    reqwest::get(link)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

async fn create_unvalid_subscriber(app: &TestApp) {
    let email = "git@github2.com";
    let link = create_unconfirmed_subscriber(app, email).await;
    let _ = sqlx::query!(
        r#"
        UPDATE subscription 
        SET email = '@github2.com' 
        WHERE email = 'git@github2.com'
        "#
    )
    .execute(&app.pool)
    .await;
    reqwest::get(link)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn valid_publish() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;
    create_unvalid_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter = serde_json::json!({
        "subject": "Publish Newsletter Test",
        "text_body": "This is someone called plain text.",
        "html_body": "<p>This is someone called html.</p>"
    });
    let res = app.publish_request(&newsletter).await;

    assert_eq!(200, res.status().as_u16());
}
