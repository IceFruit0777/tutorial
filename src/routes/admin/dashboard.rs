use actix_web::{http::header::ContentType, web, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    util::{e500, format_flash_messages, get_username_by_user_id},
};

pub async fn admin_dashboard(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = get_username_by_user_id(*user_id, &pool)
        .await
        .map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            include_str!("dashboard.html"),
            format_flash_messages(flash_messages),
            username,
        )))
}
