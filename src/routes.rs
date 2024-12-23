mod subscription;
mod subscription_confirm;

pub use subscription::*;
pub use subscription_confirm::*;

use actix_web::Responder;

pub async fn health_check() -> impl Responder {
    format!("health check passed.")
}
