use actix_web::{web, HttpResponse, Responder};
use rand::distributions::{Alphanumeric, DistString};
use sqlx::{types::chrono::Utc, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{config::Config, domain::Subscriber, email_client::EmailCient};

#[derive(serde::Deserialize)]
pub struct FormData {
    pub name: String,
    pub email: String,
}

#[tracing::instrument(
    name = "在subscription表中插入一条数据...",
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
) -> impl Responder {
    let subscriber: Subscriber = match form.0.try_into() {
        Ok(value) => value,
        Err(_) => return HttpResponse::BadRequest(),
    };

    // 开启事务
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => {
            tracing::error!("事务开启失败. {e}");
            return HttpResponse::InternalServerError();
        }
    };

    let subscriber_id = match add_subscriber(&mut transaction, &subscriber).await {
        Ok(subscriber_id) => {
            tracing::info!("数据插入成功.");
            subscriber_id
        }
        Err(_) => {
            tracing::error!("数据插入失败.");
            return HttpResponse::InternalServerError();
        }
    };

    // 生成订阅令牌(用于发送确认订阅邮件)
    let subscription_token = generate_subscription_token();
    if store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        tracing::error!("订阅令牌绑定失败.");
        return HttpResponse::InternalServerError();
    } else {
        tracing::info!("订阅令牌绑定成功.");
    }

    // 提交事务
    if transaction.commit().await.is_err() {
        tracing::error!("事务提交失败.");
        return HttpResponse::InternalServerError();
    } else {
        tracing::error!("事务提交成功.");
    }

    if send_confirm_email(&subscriber, &email_client, &config, &subscription_token)
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

/// 新增订阅者
async fn add_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber: &Subscriber,
) -> Result<Uuid, sqlx::Error> {
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
    .execute(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("failed to execute query. {e}");
        e
    })?;

    Ok(subscriber_id)
}

/// 储存订阅令牌
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_token (subscriber_id, subscription_token)
        VALUES($1, $2)
        "#,
        subscriber_id,
        subscription_token
    )
    .execute(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("failed to execute query. {e}");
        e
    })?;

    Ok(())
}

/// 发送确认订阅邮件
async fn send_confirm_email(
    subscriber: &Subscriber,
    email_client: &EmailCient,
    config: &Config,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
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
