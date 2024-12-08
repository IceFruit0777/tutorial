use actix_web::{web, HttpResponse, Responder};
use sqlx::{types::chrono::Utc, PgPool};
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "在subscription表中插入一条数据...",
    skip(form, pool),
    fields(
        %form.name,
        %form.email
    )
)]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> impl Responder {
    let query_span = tracing::info_span!("polling future...");
    match sqlx::query!(
        r#"
        INSERT INTO subscription (id, email, name, subscribed_at)
        VALUES($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.get_ref())
    .instrument(query_span)
    .await
    {
        Ok(_) => {
            tracing::info!("数据插入成功.");
            HttpResponse::Ok()
        }
        Err(e) => {
            tracing::error!("数据插入失败: {e:?}");
            HttpResponse::InternalServerError()
        }
    }
}
