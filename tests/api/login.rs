use crate::helper::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn show_flash_msg_when_login_failed() {
    let app = spawn_app().await;

    // 登录失败
    let body = serde_json::json!({
        "username": "random-username",
        "password": "random-password",
    });
    let res = app.post_login(&body).await;
    assert_is_redirect_to(&res, "/login");

    // 跟随重定向，登录页面显示登录失败信息
    let login_html = app.get_login_html().await;
    assert!(login_html.contains("<p><i>Authentication failed.</i></p>"));

    // 再次加载登录页面，登录页面不会再次显示登录失败信息
    let login_html = app.get_login_html().await;
    assert!(!login_html.contains("<p><i>Authentication failed.</i></p>"));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    let app = spawn_app().await;

    // 登录成功
    let body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let res = app.post_login(&body).await;
    assert_is_redirect_to(&res, "/admin/dashboard");

    // 跟随重定向
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}!", &app.test_user.username)));
}
