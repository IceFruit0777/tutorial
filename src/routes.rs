mod admin;
mod login;
mod subscription;
mod subscription_confirm;

pub use admin::*;
pub use login::*;
pub use subscription::*;
pub use subscription_confirm::*;

use actix_web::{HttpResponse, Responder};

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}
