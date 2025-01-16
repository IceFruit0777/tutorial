use std::fmt::Debug;

use actix_web::{http::StatusCode, web, HttpResponse, Responder, ResponseError};
use anyhow::Context;
use rand::distributions::{Alphanumeric, DistString};
use sqlx::{types::chrono::Utc, PgConnection, PgPool};
use uuid::Uuid;

use crate::{config::Config, domain::Subscriber, email_client::EmailCient, util::error_chain_fmt};

#[derive(serde::Deserialize)]
pub struct FormData {
    pub name: String,
    pub email: String,
}

#[tracing::instrument(
    name = "新增订阅者",
    skip(form, pool, email_client, config),
    fields(
        %form.name,
        %form.email
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailCient>,
    config: web::Data<Config>,
) -> Result<impl Responder, SubscribeError> {
    let subscriber: Subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;
    // 开启事务
    let mut transaction = pool
        .begin()
        .await
        .context("failed to open a transaction.")?;
    // 新增订阅者
    let subscriber_id = add_subscriber(transaction.as_mut(), &subscriber)
        .await
        .context("failed to add new subscriber in the database.")?;
    // 生成订阅令牌
    let subscription_token = generate_subscription_token();
    // 储存订阅令牌
    store_token(transaction.as_mut(), subscriber_id, &subscription_token)
        .await
        .context("failed to store the confirmation token for a new subscriber.")?;
    // 提交事务
    transaction
        .commit()
        .await
        .context("failed to commit transaction.")?;
    // 发送确认订阅邮件
    send_confirm_email(&subscriber, &email_client, &config, &subscription_token)
        .await
        .context("failed to send a confimation email.")?;

    Ok(HttpResponse::Ok())
}

/// 新增订阅者
async fn add_subscriber(
    executor: &mut PgConnection,
    subscriber: &Subscriber,
) -> sqlx::Result<Uuid> {
    let subscriber_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO subscription (id, name, email, subscribed_at, status)
        VALUES($1, $2, $3, $4, $5)
        "#,
        subscriber_id,
        subscriber.name.as_ref(),
        subscriber.email.as_ref(),
        Utc::now(),
        subscriber.status.as_str()
    )
    .execute(executor)
    .await?;

    Ok(subscriber_id)
}

/// 储存订阅令牌
async fn store_token(
    executor: &mut PgConnection,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_token (subscriber_id, subscription_token)
        VALUES($1, $2)
        "#,
        subscriber_id,
        subscription_token
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// 发送确认订阅邮件
async fn send_confirm_email(
    subscriber: &Subscriber,
    email_client: &EmailCient,
    config: &Config,
    subscription_token: &str,
) -> reqwest::Result<()> {
    let confirm_link = format!(
        "{}/subscription/confirm?subscription_token={}",
        config.web.base_url, subscription_token
    );
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

/// 生成25位随机(a-z, A-Z and 0-9)的订阅令牌
fn generate_subscription_token() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 25)
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("failed to validate form data when add a new subscriber: {0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
