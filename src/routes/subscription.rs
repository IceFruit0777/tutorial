use actix_web::{web, HttpResponse, Responder};
use sqlx::{postgres::PgQueryResult, types::chrono::Utc, PgPool};
use tracing::Instrument;
use uuid::Uuid;

use crate::{domain::Subscriber, email_client::EmailCient};

#[derive(serde::Deserialize)]
pub struct FormData {
    pub name: String,
    pub email: String,
}

#[tracing::instrument(
    name = "在subscription表中插入一条数据...",
    skip(form, pool, email_client),
    fields(
        %form.name,
        %form.email
    )
)]
#[allow(clippy::async_yields_async)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailCient>,
) -> impl Responder {
    let subscriber: Subscriber = match form.0.try_into() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest(),
    };

    if insert(&subscriber, &pool).await.is_err() {
        tracing::error!("数据插入失败.");
        return HttpResponse::InternalServerError();
    } else {
        tracing::info!("数据插入成功.");
    }

    if send_confirm_email(&subscriber, &email_client)
        .await
        .is_err()
    {
        tracing::error!("邮件发送失败.");
        return HttpResponse::InternalServerError();
    } else {
        tracing::info!("邮件发送成功.");
    }

    HttpResponse::Ok()
}

async fn insert(
    subscriber: &Subscriber,
    pool: &web::Data<PgPool>,
) -> Result<PgQueryResult, sqlx::Error> {
    let query_span = tracing::info_span!("polling future...");
    sqlx::query!(
        r#"
        INSERT INTO subscription (id, name, email, subscribed_at, status)
        VALUES($1, $2, $3, $4, 'pending_confirmation')
        "#,
        Uuid::new_v4(),
        subscriber.name.as_ref(),
        subscriber.email.as_ref(),
        Utc::now()
    )
    .execute(pool.get_ref())
    .instrument(query_span)
    .await
}

async fn send_confirm_email(
    subscriber: &Subscriber,
    email_client: &EmailCient,
) -> Result<(), reqwest::Error> {
    let confirm_link = "https://my-api.com/subscription/confirm";
    let subject = "Welcome!";
    let text_body = format!(
        "Welcome to our tutorial!\nVisit {} to confirm you subscribe.",
        &confirm_link
    );
    let html_body = format!(
        "Welcome to our tutorial!<br />\
        Click <a href=\"{}\">here</a> to confirm you subscribe.",
        &confirm_link
    );
    email_client
        .send(&subscriber.email, subject, &text_body, &html_body)
        .await
}
