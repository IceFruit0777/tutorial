use actix_web::{body::to_bytes, http::StatusCode, HttpResponse};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::IdempotencyKey;

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "header_pair")]
struct HeaderPair {
    name: String,
    value: Vec<u8>,
}

#[tracing::instrument(skip_all)]
pub async fn save_response(
    executor: &mut PgConnection,
    user_id: &Uuid,
    idempotency_key: &IdempotencyKey,
    response: HttpResponse,
) -> anyhow::Result<HttpResponse> {
    // 解析响应
    let (head, body) = response.into_parts();
    let status_code = head.status().as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(head.headers().len());
        for (name, value) in head.headers().iter() {
            let name = name.as_str().to_owned();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPair { name, value });
        }
        h
    };
    let body = to_bytes(body).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    // 持久化
    sqlx::query_unchecked!(
        r#"
        UPDATE idempotency
        SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1 AND idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref(),
    )
    .execute(executor)
    .await?;

    let response = head.set_body(body).map_into_boxed_body();
    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn get_saved_response(
    pool: &PgPool,
    user_id: &Uuid,
    idempotency_key: &IdempotencyKey,
) -> anyhow::Result<Option<HttpResponse>> {
    let row = sqlx::query!(
        r#"
        SELECT 
            response_status_code,
            response_headers AS "response_headers!: Vec<HeaderPair>",
            response_body
        FROM idempotency
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
    )
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        let status_code = StatusCode::from_u16(r.response_status_code.unwrap().try_into()?)?;
        let mut res = HttpResponse::build(status_code);
        for HeaderPair { name, value } in r.response_headers {
            res.append_header((name, value));
        }
        Ok(Some(res.body(r.response_body.unwrap())))
    } else {
        Ok(None)
    }
}

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(HttpResponse),
}

pub async fn try_processing(
    pool: &PgPool,
    user_id: &Uuid,
    idempotency_key: &IdempotencyKey,
) -> anyhow::Result<NextAction> {
    let mut transaction = pool.begin().await?;
    let n_insert_rows = sqlx::query!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        ) VALUES (
            $1, $2, now()
        )
        ON CONFLICT DO NOTHING
        "#,
        user_id,
        idempotency_key.as_ref(),
    )
    .execute(transaction.as_mut())
    .await?
    .rows_affected();

    if n_insert_rows > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(pool, user_id, idempotency_key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("expected a saved response but didn't find."))?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}
