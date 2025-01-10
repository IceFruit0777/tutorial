use uuid::Uuid;

use crate::helper::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn changing_password_success() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // 1. login
    let res = app
        .post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_is_redirect_to(&res, "/admin/dashboard");

    // 2. change password
    let body = serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_password,
        "new_password_check": &new_password,
    });
    let res = app.post_change_password(&body).await;
    assert_is_redirect_to(&res, "/login");

    // 3. follow the redirect
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>密码修改成功.</i></p>"));

    // 4. login with new password
    let res = app
        .post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": &new_password,
        }))
        .await;
    assert_is_redirect_to(&res, "/admin/dashboard");
}

#[tokio::test]
async fn you_must_sign_in_to_see_the_change_password_form() {
    let app = spawn_app().await;

    let res = app.get_change_password().await;

    assert_is_redirect_to(&res, "/login");
}

#[tokio::test]
async fn you_must_sign_in_to_change_password() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4().to_string();
    let body = serde_json::json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password": new_password,
        "new_password_check": new_password,
    });
    let res = app.post_change_password(&body).await;

    assert_is_redirect_to(&res, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;

    // 1. sign in
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // 2. try to change password
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();
    assert_ne!(new_password, another_new_password);

    let res = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &another_new_password,
        }))
        .await;
    assert_is_redirect_to(&res, "/admin/password");

    // 3. follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>两次输入的密码不一致.</i></p>"));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;

    // 1. sign in
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // 2. try to change password
    let current_password = Uuid::new_v4().to_string();
    let new_password = Uuid::new_v4().to_string();
    assert_ne!(app.test_user.password, current_password);

    let res = app
        .post_change_password(&serde_json::json!({
            "current_password": &current_password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    assert_is_redirect_to(&res, "/admin/password");

    // 3. follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>密码不正确.</i></p>"));
}
