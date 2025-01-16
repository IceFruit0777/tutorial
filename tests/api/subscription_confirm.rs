use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helper::spawn_app;

#[tokio::test]
async fn valid_confirm() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let body = "name=IceFruit%20huang&email=git%40github.com";
    app.post_subscribe(body).await;
    let confirm_link = app.get_confirmation_link().await;

    let res = reqwest::get(confirm_link).await.unwrap();
    assert_eq!(200, res.status().as_u16());
}

#[tokio::test]
async fn query_subscriber_id_error() {
    let app = spawn_app().await;
    let body = "name=IceFruit%20huang&email=git%40github.com";
    app.post_subscribe(body).await;
    let confirm_link = app.get_confirmation_link().await;

    // sabotage the database
    sqlx::query!("ALTER TABLE subscription_token DROP COLUMN subscription_token;")
        .execute(app.pool.get_ref())
        .await
        .unwrap();

    let res = reqwest::get(confirm_link).await.unwrap();
    assert_eq!(500, res.status().as_u16());
}

#[tokio::test]
async fn subscription_token_not_exist() {
    let app = spawn_app().await;

    let fake_confirm_link = app
        .web_base_url
        .join("/subscription/confirm?subscription_token=123456")
        .unwrap();
    let res = reqwest::get(fake_confirm_link).await.unwrap();

    assert_eq!(401, res.status().as_u16());
}

#[tokio::test]
async fn update_subscriber_status_error() {
    let app = spawn_app().await;
    let body = "name=IceFruit%20huang&email=git%40github.com";
    app.post_subscribe(body).await;
    let confirm_link = app.get_confirmation_link().await;

    // sabotage the database
    sqlx::query!("ALTER TABLE subscription DROP COLUMN status;")
        .execute(app.pool.get_ref())
        .await
        .unwrap();

    let res = reqwest::get(confirm_link).await.unwrap();
    assert_eq!(500, res.status().as_u16());
}
