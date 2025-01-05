use uuid::Uuid;
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

fn newsletter() -> serde_json::Value {
    serde_json::json!({
        "subject": "Publish Newsletter Test",
        "text_body": "This is someone called plain text.",
        "html_body": "<p>This is someone called html.</p>"
    })
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

    let res = app.publish_request(&newsletter()).await;

    assert_eq!(200, res.status().as_u16());
}

#[tokio::test]
async fn request_header_missing_authorization() {
    let app = spawn_app().await;

    let res = reqwest::Client::new()
        .post(app.web_base_url.join("/newsletter/publish").unwrap())
        .json(&newsletter())
        .send()
        .await
        .expect("failed to execute request.");

    assert_eq!(401, res.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        res.headers()["WWW-Authenticate"]
    );
}

#[tokio::test]
async fn user_is_not_exist() {
    let app = spawn_app().await;
    // 随机凭证
    let username = Uuid::new_v4().to_string();
    let password = Uuid::new_v4().to_string();

    let res = reqwest::Client::new()
        .post(app.web_base_url.join("/newsletter/publish").unwrap())
        .basic_auth(username, Some(password))
        .json(&newsletter())
        .send()
        .await
        .expect("failed to execute request.");

    assert_eq!(401, res.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        res.headers()["WWW-Authenticate"]
    );
}

#[tokio::test]
async fn invalid_password() {
    let app = spawn_app().await;
    let username = &app.test_user.username;
    // 随机密码
    let password = Uuid::new_v4().to_string();
    assert_ne!(password, app.test_user.password);

    let res = reqwest::Client::new()
        .post(app.web_base_url.join("/newsletter/publish").unwrap())
        .basic_auth(username, Some(password))
        .json(&newsletter())
        .send()
        .await
        .expect("failed to execute request.");

    assert_eq!(401, res.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        res.headers()["WWW-Authenticate"]
    );
}
