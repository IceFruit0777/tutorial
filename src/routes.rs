mod subscription;
mod subscription_confirm;

pub use subscription::*;
pub use subscription_confirm::*;

use actix_web::{HttpResponse, Responder};

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}

/// tracing error log
fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    write!(f, "{e}")?;
    let mut current = e.source();
    while let Some(cause) = current {
        write!(f, " Caused by: {cause}")?;
        current = cause.source();
    }
    Ok(())
}
