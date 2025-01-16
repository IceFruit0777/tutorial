use std::time::Duration;

use fake::{
    faker::{internet::en::SafeEmail, name::zh_cn::Name},
    Fake,
};
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockBuilder, ResponseTemplate,
};

use crate::helper::{assert_is_redirect_to, spawn_app, TestApp};

#[tokio::test]
async fn publish_success() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let res = app.post_publish_with_default_issue(None).await;
    assert_is_redirect_to(&res, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains("<p><i>简报已接收"));

    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn you_must_login_to_publish_issue() {
    let app = spawn_app().await;
    let res = app.post_publish_with_default_issue(None).await;
    assert_is_redirect_to(&res, "/login");
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let idempotency_key = Uuid::new_v4().to_string();
    // 1. 提交表单
    let res = app
        .post_publish_with_default_issue(Some(idempotency_key.clone()))
        .await;
    assert_is_redirect_to(&res, "/admin/dashboard");

    // 2. 跟随重定向
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains("<p><i>简报已接收"));

    // 3. 再次提交表单
    let res = app
        .post_publish_with_default_issue(Some(idempotency_key.clone()))
        .await;
    assert_is_redirect_to(&res, "/admin/dashboard");

    // 4. 跟随重定向
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains("<p><i>简报已接收"));

    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        // 设置延迟，确保第二个请求在第一个请求处理完成之前到达
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // 并发提交两个邮件表单
    let idempotency_key = Uuid::new_v4().to_string();
    let res = app.post_publish_with_default_issue(Some(idempotency_key.clone()));
    let res2 = app.post_publish_with_default_issue(Some(idempotency_key.clone()));
    let (res, res2) = tokio::join!(res, res2);

    assert_eq!(res.status(), res2.status());
    assert_eq!(res.text().await.unwrap(), res2.text().await.unwrap());

    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn email_api_exception_will_retry() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    create_confirmed_subscriber(&app).await;

    // 首次发送邮件异常
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .up_to_n_times(1)
        .mount(&app.email_server)
        .await;
    // 重试发送邮件成功
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .named("Delivery retry.")
        .expect(1)
        .up_to_n_times(1)
        .mount(&app.email_server)
        .await;

    app.post_publish_with_default_issue(None).await;
    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn issue_not_found_will_delete_whole_queue() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    create_confirmed_subscriber(&app).await;
    create_confirmed_subscriber(&app).await;

    app.post_publish_with_default_issue(None).await;

    // 清空issue
    let mut trasaction = app.pool.begin().await.unwrap();
    sqlx::query!("ALTER TABLE issue_delivery_queue DROP CONSTRAINT issue_delivery_queue_newsletter_issue_id_fkey")
        .execute(trasaction.as_mut())
        .await
        .unwrap();
    sqlx::query!("TRUNCATE TABLE newsletter_issue")
        .execute(trasaction.as_mut())
        .await
        .unwrap();
    trasaction.commit().await.unwrap();

    app.dispatch_all_pending_emails().await;
}

fn when_sending_an_email() -> MockBuilder {
    Mock::given(path("/email")).and(method("POST"))
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> reqwest::Url {
    let _mock_guard = when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&app.email_server)
        .await;

    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(&serde_json::json!({
        "name": name,
        "email": email,
    }))
    .unwrap();
    app.post_subscribe(&body).await.error_for_status().unwrap();

    app.get_confirmation_link().await
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let link = create_unconfirmed_subscriber(app).await;
    reqwest::get(link)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
