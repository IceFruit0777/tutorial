use actix_web::{web, Responder};
use actix_web_flash_messages::FlashMessage;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    authentication::UserId,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    util::{e400, e500, see_other},
    SubscriberStatus,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    subject: String,
    text_body: String,
    html_body: String,
    // 幂等键
    idempotency_key: String,
}

#[tracing::instrument(
    name = "发布newsletter issue", 
    skip_all,
    fields(user_id = % **user_id)
)]
pub async fn publish(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<impl Responder, actix_web::Error> {
    fn send_success_message() {
        FlashMessage::info(
            r#"简报已接收，邮件将很快发送给所有订阅用户，
            可<a href="\#">点击此处</a>查看详情."#,
        )
        .send();
    }

    let user_id = user_id.into_inner();
    let FormData {
        subject,
        text_body,
        html_body,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;

    let mut transaction = match try_processing(&pool, &user_id, &idempotency_key)
        .await
        .map_err(e500)?
    {
        // 第一次请求，执行全部流程
        NextAction::StartProcessing(t) => t,
        // 第二次请求
        // 等待第一次请求执行完成，响应写入数据库
        // 获取响应并返回
        NextAction::ReturnSavedResponse(saved_response) => {
            send_success_message();
            return Ok(saved_response);
        }
    };

    // 存储邮件简报
    let issue_id = insert_newsletter_issue(&mut transaction, &subject, &text_body, &html_body)
        .await
        .map_err(e500)?;
    // 新增简报发布队列
    enqueue_delivery_task(&mut transaction, &issue_id)
        .await
        .map_err(e500)?;
    // 存储响应
    let res = see_other("/admin/dashboard");
    let res = save_response(&mut transaction, &user_id, &idempotency_key, res)
        .await
        .map_err(e500)?;

    transaction.commit().await.map_err(e500)?;
    send_success_message();
    Ok(res)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    executor: &mut PgConnection,
    subject: &str,
    text_body: &str,
    html_body: &str,
) -> sqlx::Result<Uuid> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issue (
            newsletter_issue_id,
            subject,
            text_body,
            html_body,
            published_at
        ) VALUES (
            $1, $2, $3, $4, now()
        )
        "#,
        newsletter_issue_id,
        subject,
        text_body,
        html_body,
    )
    .execute(executor)
    .await?;

    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_task(
    executor: &mut PgConnection,
    newsletter_issue_id: &Uuid,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscription
        WHERE status = $2
        "#,
        newsletter_issue_id,
        SubscriberStatus::Confirmed.as_str()
    )
    .execute(executor)
    .await?;

    Ok(())
}
