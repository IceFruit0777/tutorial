use crate::helper::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn sign_in_then_redirect_to_dashboard() {
    let app = spawn_app().await;

    let res = app
        .post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_is_redirect_to(&res, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("<p>Welcome {}!</p>", &app.test_user.username)));
}

#[tokio::test]
async fn you_must_sign_in_to_access_dashboard() {
    let app = spawn_app().await;

    let res = app.get_admin_dashboard().await;

    assert_is_redirect_to(&res, "/login");
}

#[tokio::test]
async fn sign_up_clear_session_state() {
    let app = spawn_app().await;

    // 1. sign in
    app.test_user.login(&app).await;

    // 2. sign up
    let res = app.post_logout().await;
    assert_is_redirect_to(&res, "/login");

    // 3. follow redirect
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>注销成功.</i></p>"));

    // 4. try to access admin dashboard
    let res = app.get_admin_dashboard().await;
    assert_is_redirect_to(&res, "/login");
}
