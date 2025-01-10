use std::fmt::Debug;

use actix_web::{web, HttpRequest, Responder};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    domain::SubscriberEmail,
    email_client::EmailCient,
    util::{e500, see_other},
    SubscriberStatus,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    subject: String,
    text_body: String,
    html_body: String,
}

#[tracing::instrument(name = "给用户发布资讯", skip(body, pool, email_client))]
pub async fn publish(
    body: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailCient>,
    request: HttpRequest,
    user_id: web::ReqData<UserId>,
) -> Result<impl Responder, actix_web::Error> {
    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;

    for subscriber in subscribers {
        email_client
            .send(
                &subscriber.email,
                &body.subject,
                &body.text_body,
                &body.html_body,
            )
            .await
            .with_context(|| format!("failed to publish newsletter to {:?}", &subscriber.email))
            .map_err(e500)?;
    }

    FlashMessage::info("发送成功.").send();
    Ok(see_other("/admin/dashboard"))
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

/// 获取所有已确认订阅的用户
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<ConfirmedSubscriber>, anyhow::Error> {
    // 查询所有已确认订阅的用户
    let rows = sqlx::query!(
        r#"
        SELECT email FROM subscription 
        WHERE status = $1
        "#,
        SubscriberStatus::Confirmed.as_str()
    )
    .fetch_all(pool)
    .await
    .context("failed to query confirmed subscribers.")?;

    // 过滤邮件地址验证失败的用户
    // 由于程序更新迭代，邮件地址验证规则可能发生更改
    // 之前验证通过的邮件地址可能现在会验证失败
    // 这里对验证失败的用户打印一条警告日志，供开发人员排查处理
    let confirmed_subscribers: Vec<ConfirmedSubscriber> = rows
        .into_iter()
        .filter_map(|r| match SubscriberEmail::parse(&r.email) {
            Ok(email) => Some(ConfirmedSubscriber { email }),
            Err(error) => {
                tracing::warn!(
                    "A confirmed subscriber is using an invalid email address. {}",
                    error
                );
                None
            }
        })
        .collect();
    tracing::info!(
        "共有{}位邮件地址有效的已确认订阅用户.",
        &confirmed_subscribers.len()
    );

    Ok(confirmed_subscribers)
}
