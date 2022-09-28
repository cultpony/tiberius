use axum::{response::Redirect, Extension, Form, Router};
use axum_extra::routing::{RouterExt, TypedPath};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;
use tiberius_core::{
    error::TiberiusResult,
    request_helper::{HtmlResponse, RedirectResponse, TiberiusResponse},
    session::{AuthMethod, Authenticated, SessionMode, Unauthenticated},
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_dependencies::axum_flash::Flash;
use tiberius_models::{Client, User, UserLoginResult};

use crate::{
    api::v3::misc::sessho::PathApiV3MiscSessionHandover, pages::activity::PathActivityIndex,
};

pub fn session_pages(r: Router) -> Router {
    r.typed_get(new_session)
        .typed_get(forgot_password)
        .typed_post(post_new_session)
        .typed_post(post_registration)
        .typed_get(get_destroy_session)
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/sessions/new")]
pub struct PathNewSession {}

#[instrument(skip(state, rstate))]
pub async fn new_session(
    _: PathNewSession,
    Extension(state): Extension<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let mut client: Client = state.get_db_client();
    let body = html! {
        h1 { "Sign in" }

        form action=(PathNewSession{}.to_uri().to_string()) method="POST" {
            //TODO: include flash messages
            //TODO: CSRF protection
            p {
                a href=(PathSessionForgotPw{}.to_uri().to_string()) { "Forgot your password?"}
            }

            input type="hidden" name="alt_r" value="false";

            .field {
                input.input #user_email name="email" type="email" required="true" placeholder="Email" autofocus="true" pattern=".*@.*";
            }

            .field {
                input.input #user_password name="password" type="password" required="true" placeholder="Password";
            }

            .field {
                input.input #user_totp name="totp" type="text" pattern="[0-9]{6}" placeholder="TOTP";
            }

            /*.field { We don't implement session remembering, just let the session linger
                input#user_remember_me name="remember_me" type="checkbox" value="true";
                label for="user_remember_me" { "Remember me" }
            }*/

            .actions {
                button.button type="submit" { "Sign in" }
            }
        }

        p {
            strong {
                "Haven't read the "
                a href="/pages/rules" { "site rules" }
                " lately? Make sure you read them before posting or editing metadata!"
            }
        }
    };
    let page: PreEscaped<String> = html! {
        (crate::pages::common::frontmatter::app(&state, &rstate, None, &mut client, body, None).await?);
    };
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: page.into_string(),
    }))
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/session/forgot_pw")]
pub struct PathSessionForgotPw {}

#[tracing::instrument]
pub async fn forgot_password(_: PathSessionForgotPw) -> TiberiusResult<String> {
    todo!()
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/sessions/new")]
pub struct PathSessionsLogin {}

#[derive(serde::Deserialize, securefmt::Debug)]
pub struct NewSession {
    email: String,
    #[sensitive]
    password: String,
    #[sensitive]
    totp: Option<String>,
}

#[instrument(skip(state, rstate))]
pub async fn post_new_session(
    _: PathSessionsLogin,
    Extension(state): Extension<TiberiusState>,
    mut rstate: TiberiusRequestState<Unauthenticated>,
    Form(login_data): Form<NewSession>,
) -> TiberiusResult<Redirect> {
    trace!("requesting new session, verifying user");
    let mut client = state.get_db_client();

    let user: Option<User> = User::get_mail_or_name(&mut client, login_data.email.as_str()).await?;
    let retry = PathSessionsLogin {}.to_uri();
    if let Some(user) = user {
        let valid = user.validate_login(
            state.config.password_pepper(),
            &state.config.otp_secret(),
            &login_data.email,
            &login_data.password,
            login_data.totp,
        )?;
        let home = PathActivityIndex {}.to_uri();
        match valid {
            UserLoginResult::Valid => {
                let session = rstate.session_mut();
                session.set_user(&user);
                let id = session.id();
                debug!("Creating new session, persisting {} to DB", id);
                rstate.flash_mut().info("Login successfull!");
                rstate.db_session_mut().set_longterm(true).await;
                rstate.db_session_mut().set_store(true).await;
                rstate.push_session_update().await;
                Ok(Redirect::to(
                    PathActivityIndex {}.to_uri().to_string().as_str(),
                ))
            }
            UserLoginResult::Invalid => {
                debug!("password disagree");
                rstate.flash_mut().error("User or password incorrect");
                Ok(Redirect::to(retry.to_string().as_str()))
            }
            UserLoginResult::RetryWithTOTP => {
                debug!("password agree, TOTP missing");
                rstate.flash_mut().error("User or password incorrect");
                Ok(Redirect::to(retry.to_string().as_str()))
            }
        }
    } else {
        trace!("user not found");
        rstate.flash_mut().error("User or password incorrect");
        Ok(Redirect::to(retry.to_string().as_str()))
    }
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/sessions/register")]
pub struct PathRegistration {}

#[tracing::instrument]
pub async fn post_registration(_: PathRegistration) -> TiberiusResult<String> {
    todo!()
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/sessions/logout")]
pub struct PathSessionLogout {}

#[instrument(skip(state, rstate))]
pub async fn get_destroy_session(
    _: PathSessionLogout,
    Extension(state): Extension<TiberiusState>,
    mut rstate: TiberiusRequestState<Authenticated>,
) -> TiberiusResult<Redirect> {
    let session = rstate.session_mut();
    session.unset_user();
    rstate.push_session_update().await;
    rstate.db_session_mut().destroy().await;
    rstate.flash_mut().info("You have been logged out");
    Ok(Redirect::to(
        PathActivityIndex {}.to_uri().to_string().as_str(),
    ))
}
