use actix_web::{web, HttpResponse, Responder};
use sqlx::PgPool;
use uuid::Uuid;

use crate::SubscriberStatus;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "用户点击邮件中的确认订阅链接...", skip(parameters, pool))]
pub async fn subscription_confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    let subscriber_id = match get_subscriber_id(&pool, &parameters.subscription_token).await {
        Ok(id) => id,
        Err(_) => {
            tracing::error!("`subscriber_id`查询失败.");
            return HttpResponse::InternalServerError();
        }
    };

    match subscriber_id {
        None => {
            tracing::error!("订阅令牌不存在.");
            HttpResponse::Unauthorized()
        }
        Some(id) => {
            if confirm_subscriber(&pool, id).await.is_err() {
                tracing::error!("用户状态更新失败.");
                HttpResponse::InternalServerError()
            } else {
                tracing::info!("用户状态更新成功.");
                HttpResponse::Ok()
            }
        }
    }
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
    .await
    .map_err(|e| {
        tracing::error!("failed to execute query. {e:?}");
        e
    })?;

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
    .await
    .map_err(|e| {
        tracing::error!("failed to execute query. {e}");
        e
    })?;

    Ok(())
}
