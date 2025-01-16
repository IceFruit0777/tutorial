use std::time::Duration;

use actix_web::web;
use sqlx::{PgConnection, PgPool};
use tracing::field::{display, Empty};
use uuid::Uuid;

use crate::{domain::SubscriberEmail, email_client::EmailCient};

struct IssueDeliveryTask {
    issue_id: Uuid,
    email: String,
}

struct NewsletterIssue {
    subject: String,
    text_body: String,
    html_body: String,
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

pub async fn run(pool: web::Data<PgPool>, email_client: web::Data<EmailCient>) {
    loop {
        match try_execute_task(pool.as_ref(), email_client.as_ref()).await {
            Ok(ExecutionOutcome::EmptyQueue) => tokio::time::sleep(Duration::from_secs(10)).await,
            Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id = Empty,
        subscriber_email = Empty,
    ),
    err
)]
/// 尝试执行邮件简报发送任务
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailCient,
) -> anyhow::Result<ExecutionOutcome> {
    let mut transaction = pool.begin().await?;

    // 从任务队列中获取一个任务
    let task = get_and_lock_task(&mut transaction).await?;
    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }
    let issue_task = task.unwrap();
    tracing::Span::current()
        .record("newsletter_issue_id", display(&issue_task.issue_id))
        .record("subscriber_email", display(&issue_task.email));

    // 验证邮箱的有效性
    // 若无效，删除该任务
    let subscriber_email = match SubscriberEmail::parse(&issue_task.email) {
        Ok(email) => email,
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "The email address is no longer valid, subscriber_email = {}",
                &issue_task.email,
            );
            dequeue_task(&mut transaction, &issue_task).await?;
            transaction.commit().await?;
            return Err(anyhow::anyhow!(e));
        }
    };

    // 获取待发布的issue
    // 若发生Err: [`sqlx::Error::RowNotFound`]
    // 执行[`dequeue_tasks_by_issue_id`]
    let issue = match get_issue(pool, &issue_task.issue_id).await {
        Ok(issue) => issue,
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Try to get issue but not found, issue_id = {}",
                &issue_task.issue_id,
            );
            dequeue_tasks_by_issue_id(&mut transaction, &issue_task.issue_id).await?;
            transaction.commit().await?;
            return Err(e.into());
        }
    };

    // 发送邮件简报
    email_client
        .send(
            &subscriber_email,
            &issue.subject,
            &issue.text_body,
            &issue.html_body,
        )
        .await?;

    // 执行完成，删除任务
    dequeue_task(&mut transaction, &issue_task).await?;

    transaction.commit().await?;

    Ok(ExecutionOutcome::TaskCompleted)
}

#[tracing::instrument(skip_all)]
async fn get_and_lock_task(executor: &mut PgConnection) -> sqlx::Result<Option<IssueDeliveryTask>> {
    let row = sqlx::query!(
        r#"
        SELECT
            newsletter_issue_id,
            subscriber_email
        FROM
            issue_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#
    )
    .fetch_optional(executor)
    .await?;

    if let Some(row) = row {
        Ok(Some(IssueDeliveryTask {
            issue_id: row.newsletter_issue_id,
            email: row.subscriber_email,
        }))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all)]
async fn dequeue_task(
    executor: &mut PgConnection,
    issue_task: &IssueDeliveryTask,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            newsletter_issue_id = $1 AND
            subscriber_email = $2
        "#,
        issue_task.issue_id,
        issue_task.email,
    )
    .execute(executor)
    .await?;

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn dequeue_tasks_by_issue_id(
    executor: &mut PgConnection,
    issue_id: &Uuid,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            newsletter_issue_id = $1
        "#,
        issue_id,
    )
    .execute(executor)
    .await?;

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn get_issue(pool: &PgPool, issue_id: &Uuid) -> sqlx::Result<NewsletterIssue> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT
            subject, text_body, html_body
        FROM
            newsletter_issue
        WHERE
            newsletter_issue_id = $1
        "#,
        issue_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(issue)
}
