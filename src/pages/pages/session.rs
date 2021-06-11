use maud::Markup;
use philomena_models::User;

use crate::{app::HTTPReq, pages::common::flash::{Flash, put_flash}, request_helper::{ApiFormData, FormMethod, SafeSqlxRequestExt}};
use anyhow::Result;

#[get("/sessions/login")]
pub async fn new_session(req: HTTPReq) -> tide::Result {
    todo!()
}

#[post("/sessions/login")]
pub async fn new_session_post(mut req: HTTPReq) -> tide::Result {
    #[derive(serde::Deserialize)]
    struct LoginForm {
        username: String,
        password: String,
        totp: String,
    }
    let mut client = req.get_db_client().await?;
    let data: ApiFormData<LoginForm> = req.body_form().await?;
    if !data.verify_csrf(FormMethod::Create) {
        put_flash(&mut req, Flash::error("Login failed, CSRF invalid, please try again"))?;
        return Ok(tide::Redirect::new("/sessions/login").into());
    }
    let user = User::get_mail_or_name(&mut client, data.data.username).await?;
    if let Some(user) = user {
        // TODO: verify user password
        // TODO: only allow staff to login for Alpha (maybe site config?)
        req.session_mut().insert("user", user.id)?;
        put_flash(&mut req, Flash::info(format!("Welcome back {}", user.name)))?;
        return Ok(tide::Redirect::new("/").into());
    } else {
        put_flash(&mut req, Flash::error("Login failed, User or password wrong"))?;
        return Ok(tide::Redirect::new("/sessions/login").into());
    }
}

#[post("/sessions/login/totp")]
pub async fn new_session_totp(req: HTTPReq) -> tide::Result {
    todo!()
}

#[post("/sessions/register")]
pub async fn registration(req: HTTPReq) -> tide::Result {
    todo!()
}

#[post("/session/logout")]
pub async fn destroy_session(mut req: HTTPReq) -> tide::Result {
    let session = req.session_mut();
    session.destroy();
    Ok(tide::Redirect::new("/").into())
}