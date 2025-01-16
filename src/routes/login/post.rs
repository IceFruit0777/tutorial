use std::fmt::Debug;

use actix_web::{body::BoxBody, web, HttpResponse, Responder, ResponseError};
use actix_web_flash_messages::FlashMessage;
use secrecy::SecretString;
use sqlx::PgPool;

use crate::{
    authentication::{validate_credential, AuthError, Credential},
    session_state::TypedSession,
    util::error_chain_fmt,
    util::see_other,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[tracing::instrument(
    skip_all,
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<impl Responder, LoginError> {
    let credential = Credential {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current().record("username", tracing::field::display(&credential.username));

    let user_id = validate_credential(credential, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredential(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    session.renew();
    session
        .insert_user_id(user_id)
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    Ok(see_other("/admin/dashboard"))
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong.")]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse<BoxBody> {
        FlashMessage::error(self.to_string()).send();
        see_other("/login")
    }
}
