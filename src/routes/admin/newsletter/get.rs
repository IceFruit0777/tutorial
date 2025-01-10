use actix_web::{http::header::ContentType, HttpResponse, Responder};
use actix_web_flash_messages::IncomingFlashMessages;

use crate::util::format_flash_messages;

pub async fn publish_form(flash_messages: IncomingFlashMessages) -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            include_str!("newsletter.html"),
            format_flash_messages(flash_messages)
        ))
}
