use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helper::{assert_is_redirect_to, spawn_app, TestApp};

#[tokio::test]
async fn publish_success() {
    let app = spawn_app().await;

    app.post_login_with_valid_user().await;

    create_confirmed_subscriber(&app).await;
    create_unvalid_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let res = app.post_publish_with_default_issue().await;
    assert_is_redirect_to(&res, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains("<p><i>发送成功.</i></p>"));
}

#[tokio::test]
async fn you_must_login_to_publish_issue() {
    let app = spawn_app().await;

    let res = app.post_publish_with_default_issue().await;

    assert_is_redirect_to(&res, "/login");
}

async fn create_unconfirmed_subscriber(app: &TestApp, email: &str) -> reqwest::Url {
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&app.email_server)
        .await;

    let body = format!("name=IceFruit%20huang&email={email}");
    app.post_subscribe(&body).await.error_for_status().unwrap();

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
