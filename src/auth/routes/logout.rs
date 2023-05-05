use actix_web::{web::Data, get, HttpResponse, Error};
use time::OffsetDateTime;

use crate::auth::{user::UserEx, routes::build_session_cookie, SessionManager, SessionEx};

#[get("/auth/logout")]
pub async fn logout(
    sm: Data<SessionManager>,
    user: UserEx,
    session: SessionEx,
) -> Result<HttpResponse, Error> {
    if !sm.erase_session(&user, &session) {
        // either the session was already deleted or the user is trying to delete someone else's session
        return Ok(
            HttpResponse::BadRequest().body("session_delete_failed")
        )
    };

    let death_cookie = build_session_cookie("NO_SESSION_DELETE_ME")
        .expires(OffsetDateTime::from_unix_timestamp(0).unwrap())
        .finish();

    Ok(HttpResponse::Ok().cookie(death_cookie).body("success"))
}