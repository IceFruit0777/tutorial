use std::fmt::Debug;

use actix_web::{
    http::{
        header::{self, HeaderMap, HeaderValue},
        StatusCode,
    },
    web, HttpRequest, HttpResponse, Responder, ResponseError,
};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::{engine, Engine};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::SubscriberEmail, email_client::EmailCient, telemetry::spawn_blocking_with_tracing,
    SubscriberStatus,
};

#[derive(serde::Deserialize)]
pub struct BodyData {
    subject: String,
    text_body: String,
    html_body: String,
}

#[tracing::instrument(
    name = "给用户发布资讯", 
    skip(body, pool, email_client),
    fields(
        username = tracing::field::Empty,
        user_id = tracing::field::Empty,
    ),
)]
pub async fn publish(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailCient>,
    request: HttpRequest,
) -> Result<impl Responder, PublishError> {
    let credential = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", &credential.username);
    let user_id = validate_credential(credential, &pool).await?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&pool).await?;

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
            .map_err(PublishError::UnexpectedError)?;
    }

    Ok(HttpResponse::Ok())
}

struct Credential {
    username: String,
    password: SecretString,
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credential, anyhow::Error> {
    // 解码`Authorization`请求头，其必须是一个有效的UTF8字符串
    let authorization = headers
        .get("Authorization")
        .context("The `Authorization` header was missing.")?
        .to_str()
        .context("The `Authorization` header was not a valid UTF8 string.")?;
    let base64_encoded_segment = authorization
        .strip_prefix("Basic ")
        .context("The authorization scheme was not `Basic`.")?;
    let decoded_bytes = engine::general_purpose::STANDARD
        .decode(base64_encoded_segment)
        .context("failed to base64-decode `Basic` credential.")?;
    let decoded_credential =
        String::from_utf8(decoded_bytes).context("The decoded credential is not valid UTF8.")?;

    // 获取username和password
    let mut credential = decoded_credential.splitn(2, ':');
    let username = credential
        .next()
        .ok_or(anyhow::anyhow!(
            "A username must be provided in `Basic` auth."
        ))?
        .to_string();
    let password = credential.next().ok_or(anyhow::anyhow!(
        "A password must be provided in `Basic` auth."
    ))?;
    let password = SecretString::from(password);

    Ok(Credential { username, password })
}

#[tracing::instrument(name = "Validate credential", skip(credential, pool))]
/// 校验管理员凭证
async fn validate_credential(
    credential: Credential,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let mut user_id = None;
    // 预设默认密码`hunter2`
    let mut expected_password_hash = SecretString::from(
        "$argon2id$v=19$m=65536,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"   
    );

    // 管理员凭证查询成功后覆盖预设值
    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credential(&credential.username, pool)
            .await
            .map_err(PublishError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    };

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credential.password)
    })
    .await
    .context("failed to spawn blocking task.")
    .map_err(PublishError::UnexpectedError)??;

    // 只有在存储中找到凭证，才会将`user_id`设置为`Some`
    // 因此，即使默认密码与表单输入的密码匹配
    // 也永远不会对不存在的管理员校验通过
    user_id.ok_or(PublishError::AuthError(anyhow::anyhow!(
        "Unknown username."
    )))
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
/// 校验管理员密码
fn verify_password_hash(
    expected_password_hash: SecretString,
    password_candidate: SecretString,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("failed to parse hash in PHC string format.")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(PublishError::AuthError)
}

#[tracing::instrument(name = "Get stored credential", skip(username, pool))]
/// 获取管理员凭证
async fn get_stored_credential(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(Uuid, SecretString)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("failed to perform a query to retrieve stored credential.")?
    .map(|row| (row.user_id, SecretString::from(row.password_hash)));

    Ok(row)
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

/// 获取所有已确认订阅的用户
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<ConfirmedSubscriber>, PublishError> {
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
    .context("failed to query confirmed subscribers.")
    .map_err(PublishError::UnexpectedError)?;

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

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
}

impl Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        super::error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => {
                let mut res = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let authenticate = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                res.headers_mut()
                    .insert(header::WWW_AUTHENTICATE, authenticate);

                res
            }
        }
    }
}
