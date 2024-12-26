use std::fmt::Debug;

use actix_web::{http::StatusCode, web, HttpResponse, Responder, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::SubscriberStatus;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "用户点击邮件中的确认订阅链接...", 
    skip(parameters, pool),
    fields( %parameters.subscription_token )
)]
pub async fn subscription_confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder, SubscriptionConfirmError> {
    let subscriber_id = get_subscriber_id(&pool, &parameters.subscription_token)
        .await
        .context(
            "failed to query subscriber_id in table[subscription_token] with subscription_token.",
        )?;

    match subscriber_id {
        None => Err(SubscriptionConfirmError::AuthorizationError(
            "cannot find record in table[subscription_token] with subscription_token.".into(),
        ))?,
        Some(id) => confirm_subscriber(&pool, id)
            .await
            .context("failed to update subscriber's status in table[subscription] with id.")?,
    }

    Ok(HttpResponse::Ok())
}

/// 根据`subscription_token`查询`subscriber_id`
async fn get_subscriber_id(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let record = sqlx::query!(
        r#"
            SELECT subscriber_id FROM subscription_token 
            WHERE subscription_token = $1
            "#,
        subscription_token
    )
    .fetch_optional(pool)
    .await?;

    Ok(record.map(|r| r.subscriber_id))
}

/// 更新用户状态为`SubscriberStatus::Confirmed`
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscription SET status = $1 
        WHERE id = $2
        "#,
        SubscriberStatus::Confirmed.as_str(),
        subscriber_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(thiserror::Error)]
pub enum SubscriptionConfirmError {
    #[error("failed to confirm subscription: {0}")]
    AuthorizationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for SubscriptionConfirmError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            SubscriptionConfirmError::AuthorizationError(_) => StatusCode::UNAUTHORIZED,
            SubscriptionConfirmError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl Debug for SubscriptionConfirmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        super::error_chain_fmt(self, f)
    }
}
