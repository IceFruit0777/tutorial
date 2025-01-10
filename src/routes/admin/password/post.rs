use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;

use crate::{
    authentication::{validate_credential, AuthError, Credential, UserId},
    routes::admin::logout::do_logout,
    session_state::TypedSession,
    util::{e500, get_username_by_user_id, see_other},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: SecretString,
    new_password: SecretString,
    new_password_check: SecretString,
}

#[tracing::instrument(name = "更改管理员密码", skip(form, pool, session))]
pub async fn change_password(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    session: TypedSession,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();

    // 密码校验失败
    // 1. 两次输入的新密码不一致
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error("两次输入的密码不一致.").send();
        return Ok(see_other("/admin/password"));
    }
    // 2. 新密码不符合规则

    // 3. 当前密码比对不一致
    let username = get_username_by_user_id(*user_id, &pool)
        .await
        .map_err(e500)?;
    let credential = Credential {
        username,
        password: form.0.current_password,
    };
    if let Err(e) = validate_credential(credential, &pool).await {
        match e {
            AuthError::InvalidCredential(_) => {
                FlashMessage::error("密码不正确.").send();
                return Ok(see_other("/admin/password"));
            }
            AuthError::UnexpectedError(_) => return Err(e500(e)),
        }
    };

    // 更新密码
    crate::authentication::change_password(*user_id, form.0.new_password, &pool)
        .await
        .map_err(e500)?;

    // 登出
    FlashMessage::info("密码修改成功.").send();
    do_logout(session)
}
