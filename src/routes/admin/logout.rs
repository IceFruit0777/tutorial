use actix_web_flash_messages::FlashMessage;

use crate::{session_state::TypedSession, util::see_other};

pub async fn logout(session: TypedSession) -> Result<actix_web::HttpResponse, actix_web::Error> {
    FlashMessage::info("注销成功.").send();
    do_logout(session)
}

pub fn do_logout(session: TypedSession) -> Result<actix_web::HttpResponse, actix_web::Error> {
    session.logout();
    Ok(see_other("/login"))
}
