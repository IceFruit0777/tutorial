use tutorial::SubscriberStatus;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helper::spawn_app;

#[tokio::test]
async fn valid_subscribe() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let body = "name=IceFruit%20huang&email=git%40github.com";
    let res = app.post_subscribe(body).await;
    assert_eq!(200, res.status().as_u16());

    let record = sqlx::query!("select name, email, status from subscription")
        .fetch_one(app.pool.get_ref())
        .await
        .expect("failed to execute query.");
    assert_eq!("IceFruit huang", record.name);
    assert_eq!("git@github.com", record.email);
    assert_eq!(
        SubscriberStatus::PendingConfirmation.as_str(),
        record.status
    );
}

#[tokio::test]
async fn illegal_fields() {
    let app = spawn_app().await;
    let datas = [
        ("name=&email=git%40github01.com", "name is empty."),
        ("name=IceFruit%20huang&email=", "email is empty."),
    ];
    for (body, payload) in datas {
        let res = app.post_subscribe(body).await;
        assert_eq!(400, res.status().as_u16(), "{payload}");
    }
}

#[tokio::test]
async fn add_subscriber_error() {
    let app = spawn_app().await;
    let body = "name=IceFruit%20huang&email=git%40github.com";

    // sabotage the database
    sqlx::query!("ALTER TABLE subscription DROP COLUMN name;")
        .execute(app.pool.get_ref())
        .await
        .unwrap();

    let res = app.post_subscribe(body).await;
    assert_eq!(500, res.status().as_u16());
}

#[tokio::test]
async fn store_token_error() {
    let app = spawn_app().await;
    let body = "name=IceFruit%20huang&email=git%40github.com";

    // sabotage the database
    sqlx::query!("ALTER TABLE subscription_token DROP COLUMN subscription_token;")
        .execute(app.pool.get_ref())
        .await
        .unwrap();

    let res = app.post_subscribe(body).await;
    assert_eq!(500, res.status().as_u16());
}
